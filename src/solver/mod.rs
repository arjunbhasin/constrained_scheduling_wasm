use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use chrono::{Duration as ChronoDuration, NaiveDateTime};
use serde_with::serde_as;
use serde_with::TimestampMilliSeconds;
use rand::prelude::*;

#[serde_as]
#[derive(Clone, Debug, Deserialize)]
pub struct Break {
    #[serde_as(as = "TimestampMilliSeconds<i64>")]
    pub from: NaiveDateTime,
    #[serde_as(as = "TimestampMilliSeconds<i64>")]
    pub to: NaiveDateTime,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Driver {
    pub id: String,
    pub breaks: Option<Vec<Break>>,
    pub preference: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Vehicle {
    pub id: String,
    pub tags: Option<Vec<String>>,
    pub max_weight: f64,
    pub max_volume: Option<f64>,
}

#[serde_as]
#[derive(Clone, Debug, Deserialize)]
pub struct Order {
    pub id: String,
    #[serde_as(as = "TimestampMilliSeconds<i64>")]
    pub start_time: NaiveDateTime,
    #[serde_as(as = "TimestampMilliSeconds<i64>")]
    pub end_time: NaiveDateTime,
    pub priority: Option<u32>,
    pub tags: Option<Vec<String>>,
    pub weight: f64,
    pub volume: Option<f64>,
}



#[derive(Clone, Debug, Serialize)]
pub struct Assignment {
    pub order_id: String,
    pub driver_id: String,
    pub vehicle_id: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct SchedulingResponse {
    pub assignments: Vec<Assignment>,
}

#[derive(Clone)]
pub struct SolverState {
    driver_schedules: HashMap<String, Vec<(NaiveDateTime, NaiveDateTime)>>,
    vehicle_schedules: HashMap<String, Vec<(NaiveDateTime, NaiveDateTime)>>,
    pub assignments: Vec<Assignment>,
    pub score: f64,
}

impl SolverState {
    fn new(drivers: &Vec<Driver>, vehicles: &Vec<Vehicle>) -> SolverState {
        let driver_schedules = drivers
            .iter()
            .map(|d| (d.id.clone(), Vec::new()))
            .collect::<HashMap<_, _>>();
        let vehicle_schedules = vehicles
            .iter()
            .map(|v| (v.id.clone(), Vec::new()))
            .collect::<HashMap<_, _>>();
        SolverState {
            driver_schedules,
            vehicle_schedules,
            assignments: Vec::new(),
            score: 0.0,
        }
    }

    fn assign_order(
        &mut self,
        order: &Order,
        driver: &Driver,
        vehicle: &Vehicle,
        priority_map: &HashMap<String, u32>,
    ) {
        self.driver_schedules
            .get_mut(&driver.id)
            .unwrap()
            .push((order.start_time, order.end_time));
        self.vehicle_schedules
            .get_mut(&vehicle.id)
            .unwrap()
            .push((order.start_time, order.end_time));
        self.assignments.push(Assignment {
            order_id: order.id.clone(),
            driver_id: driver.id.clone(),
            vehicle_id: vehicle.id.clone(),
        });
        let mut weight = *priority_map.get(&order.id).unwrap_or(&1) as f64;
        const PREFERENCE_BONUS: f64 = 0.1;
        if let Some(preferred_vehicle) = &driver.preference {
            if &vehicle.id == preferred_vehicle {
                weight += PREFERENCE_BONUS;
            }
        }
        self.score += weight;
    }

    fn calculate_score(&self, priority_map: &HashMap<String, u32>, drivers: &Vec<Driver>) -> f64 {
        let mut score = 0.0;
        for assignment in &self.assignments {
            let priority = *priority_map.get(&assignment.order_id).unwrap_or(&1) as f64;
            let driver = drivers
                .iter()
                .find(|d| d.id == assignment.driver_id)
                .unwrap();
            let mut weight = priority;
            const PREFERENCE_BONUS: f64 = 0.1;
            if let Some(preferred_vehicle) = &driver.preference {
                if &assignment.vehicle_id == preferred_vehicle {
                    weight += PREFERENCE_BONUS;
                }
            }
            score += weight;
        }
        score
    }
}

// Helper functions
fn orders_overlap(order1: &Order, order2: &Order) -> bool {
    order1.start_time < order2.end_time && order2.start_time < order1.end_time
}

fn insufficient_break(
    order1: &Order,
    order2: &Order,
    mandatory_break: ChronoDuration,
) -> bool {
    if order1.end_time <= order2.start_time {
        order2.start_time - order1.end_time < mandatory_break
    } else if order2.end_time <= order1.start_time {
        order1.start_time - order2.end_time < mandatory_break
    } else {
        false
    }
}

fn is_driver_on_break(driver: &Driver, time: NaiveDateTime) -> bool {
    if let Some(breaks) = &driver.breaks {
        for b in breaks {
            if b.from <= time && time < b.to {
                return true;
            }
        }
    }
    false
}

fn can_assign_driver(
    order: &Order,
    driver: &Driver,
    driver_schedule: &Vec<(NaiveDateTime, NaiveDateTime)>,
    mandatory_break: ChronoDuration,
) -> bool {
    if is_driver_on_break(driver, order.start_time) {
        return false;
    }

    for &(start, end) in driver_schedule {
        // Create temporary orders for comparison
        let existing_order = Order {
            id: "".to_string(),
            start_time: start,
            end_time: end,
            priority: None,
            tags: None,
            weight: 0.0,
            volume: None,
        };

        // Check for overlapping intervals
        if orders_overlap(&existing_order, order) {
            return false;
        }
        // Check for insufficient break
        if insufficient_break(&existing_order, order, mandatory_break) {
            return false;
        }
    }
    true
}

fn can_assign_vehicle(
    order: &Order,
    vehicle: &Vehicle,
    vehicle_schedule: &Vec<(NaiveDateTime, NaiveDateTime)>,
    mandatory_break: ChronoDuration,
) -> bool {
    for &(start, end) in vehicle_schedule {
        let existing_order = Order {
            id: "".to_string(),
            start_time: start,
            end_time: end,
            priority: None,
            tags: None,
            weight: 0.0,
            volume: None,
        };

        // Check for overlapping intervals
        if orders_overlap(&existing_order, order) {
            return false;
        }
        // Check for insufficient break
        if insufficient_break(&existing_order, order, mandatory_break) {
            return false;
        }
    }

    // Check capacity constraints
    if order.weight > vehicle.max_weight {
        return false;
    }
    if let Some(order_volume) = order.volume {
        if let Some(vehicle_volume) = vehicle.max_volume {
            if order_volume > vehicle_volume {
                return false;
            }
        } else {
            return false;
        }
    }

    // Check tags
    if let Some(order_tags) = &order.tags {
        if let Some(vehicle_tags) = &vehicle.tags {
            let vehicle_tags_set: HashSet<_> = vehicle_tags.iter().collect();
            if !order_tags.iter().all(|tag| vehicle_tags_set.contains(tag)) {
                return false;
            }
        } else {
            return false;
        }
    }

    true
}

fn can_assign(
    order: &Order,
    driver: &Driver,
    vehicle: &Vehicle,
    driver_schedule: &Vec<(NaiveDateTime, NaiveDateTime)>,
    vehicle_schedule: &Vec<(NaiveDateTime, NaiveDateTime)>,
    mandatory_break: ChronoDuration,
) -> bool {
    // Check driver availability
    if !can_assign_driver(order, driver, driver_schedule, mandatory_break) {
        return false;
    }

    // Check vehicle availability and constraints
    if !can_assign_vehicle(order, vehicle, vehicle_schedule, mandatory_break) {
        return false;
    }

    true
}


pub fn initialize_random_state(
    drivers: &Vec<Driver>,
    vehicles: &Vec<Vehicle>,
    orders: &Vec<Order>,
    priority_map: &HashMap<String, u32>,
    mandatory_break: ChronoDuration,
) -> SolverState {
    let mut state = SolverState::new(drivers, vehicles);
    let mut rng = rand::thread_rng();
    let mut orders_shuffled = orders.clone();
    orders_shuffled.shuffle(&mut rng);

    for order in &orders_shuffled {
        let mut possible_assignments = Vec::new();

        for driver in drivers {
            let driver_schedule = &state.driver_schedules[&driver.id];
            if !can_assign_driver(order, driver, driver_schedule, mandatory_break) {
                continue;
            }

            for vehicle in vehicles {
                let vehicle_schedule = &state.vehicle_schedules[&vehicle.id];
                if !can_assign_vehicle(order, vehicle, vehicle_schedule, mandatory_break) {
                    continue;
                }

                if can_assign(
                    order,
                    driver,
                    vehicle,
                    driver_schedule,
                    vehicle_schedule,
                    mandatory_break,
                ) {
                    possible_assignments.push((driver.clone(), vehicle.clone()));
                }
            }
        }

        if !possible_assignments.is_empty() {
            let (driver, vehicle) = possible_assignments.choose(&mut rng).unwrap();
            state.assign_order(order, driver, vehicle, priority_map);
        }
    }

    state.score = state.calculate_score(priority_map, drivers);
    state
}



pub fn select_parent<'a>(population: &'a Vec<SolverState>) -> &'a SolverState {
    // Tournament selection
    let mut rng = rand::thread_rng();
    let tournament_size = 3;
    let mut best = population.choose(&mut rng).unwrap();
    for _ in 1..tournament_size {
        let contender = population.choose(&mut rng).unwrap();
        if contender.score > best.score {
            best = contender;
        }
    }
    best
}

pub fn crossover(
    parent1: &SolverState,
    parent2: &SolverState,
    drivers: &Vec<Driver>,
    vehicles: &Vec<Vehicle>,
    orders: &Vec<Order>,
    priority_map: &HashMap<String, u32>,
    mandatory_break: ChronoDuration,
    rng: &mut ThreadRng,
) -> SolverState {
    let mut child = SolverState::new(drivers, vehicles);

    for order in orders {
        let assignment = if rng.gen_bool(0.5) {
            parent1.assignments.iter().find(|a| a.order_id == order.id)
        } else {
            parent2.assignments.iter().find(|a| a.order_id == order.id)
        };

        if let Some(assignment) = assignment {
            let driver = drivers.iter().find(|d| d.id == assignment.driver_id).unwrap();
            let vehicle = vehicles.iter().find(|v| v.id == assignment.vehicle_id).unwrap();

            if can_assign(
                order,
                driver,
                vehicle,
                &child.driver_schedules[&driver.id],
                &child.vehicle_schedules[&vehicle.id],
                mandatory_break,
            ) {
                child.assign_order(order, driver, vehicle, priority_map);
            }
        }
    }

    // Attempt to assign unassigned orders
    for order in orders {
        if child.assignments.iter().any(|a| a.order_id == order.id) {
            continue;
        }
        let mut possible_assignments = Vec::new();
        for driver in drivers {
            let driver_schedule = &child.driver_schedules[&driver.id];
            if !can_assign_driver(order, driver, driver_schedule, mandatory_break) {
                continue;
            }
            for vehicle in vehicles {
                let vehicle_schedule = &child.vehicle_schedules[&vehicle.id];
                if !can_assign_vehicle(order, vehicle, vehicle_schedule, mandatory_break) {
                    continue;
                }
                if can_assign(
                    order,
                    driver,
                    vehicle,
                    driver_schedule,
                    vehicle_schedule,
                    mandatory_break,
                ) {
                    possible_assignments.push((driver.clone(), vehicle.clone()));
                }
            }
        }
        if !possible_assignments.is_empty() {
            let (driver, vehicle) = possible_assignments.choose(rng).unwrap();
            child.assign_order(order, driver, vehicle, priority_map);
        }
    }

    child.score = child.calculate_score(priority_map, drivers);
    child
}

pub fn mutate(
    individual: &mut SolverState,
    drivers: &Vec<Driver>,
    vehicles: &Vec<Vehicle>,
    orders: &Vec<Order>,
    priority_map: &HashMap<String, u32>,
    mandatory_break: ChronoDuration,
    rng: &mut ThreadRng,
) {
    // Randomly remove some assignments
    let remove_count = rng.gen_range(1..=3);
    for _ in 0..remove_count {
        if individual.assignments.is_empty() {
            break;
        }
        let idx = rng.gen_range(0..individual.assignments.len());
        let assignment = individual.assignments.remove(idx);
        individual.driver_schedules.get_mut(&assignment.driver_id).unwrap().retain(|&(start, end)| {
            !(start == orders.iter().find(|o| o.id == assignment.order_id).unwrap().start_time && end == orders.iter().find(|o| o.id == assignment.order_id).unwrap().end_time)
        });
        individual.vehicle_schedules.get_mut(&assignment.vehicle_id).unwrap().retain(|&(start, end)| {
            !(start == orders.iter().find(|o| o.id == assignment.order_id).unwrap().start_time && end == orders.iter().find(|o| o.id == assignment.order_id).unwrap().end_time)
        });
    }

    // Try to reassign the orders
    for order in orders {
        if individual.assignments.iter().any(|a| a.order_id == order.id) {
            continue;
        }
        let mut possible_assignments = Vec::new();
        for driver in drivers {
            let driver_schedule = &individual.driver_schedules[&driver.id];
            if !can_assign_driver(order, driver, driver_schedule, mandatory_break) {
                continue;
            }
            for vehicle in vehicles {
                let vehicle_schedule = &individual.vehicle_schedules[&vehicle.id];
                if !can_assign_vehicle(order, vehicle, vehicle_schedule, mandatory_break) {
                    continue;
                }
                if can_assign(
                    order,
                    driver,
                    vehicle,
                    driver_schedule,
                    vehicle_schedule,
                    mandatory_break,
                ) {
                    possible_assignments.push((driver.clone(), vehicle.clone()));
                }
            }
        }
        if !possible_assignments.is_empty() {
            let (driver, vehicle) = possible_assignments.choose(rng).unwrap();
            individual.assign_order(order, driver, vehicle, priority_map);
        }
    }

    individual.score = individual.calculate_score(priority_map, drivers);
}
