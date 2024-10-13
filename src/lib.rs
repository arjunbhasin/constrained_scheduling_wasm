// src/lib.rs
mod solver;
use wasm_bindgen::prelude::*;
use serde_wasm_bindgen::{from_value, to_value};
use js_sys::Function;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use rand::prelude::*;
use chrono::Duration as ChronoDuration;
use solver::{
    Driver, 
    Order, 
    SchedulingResponse,
    Vehicle,
    SolverState,
    initialize_random_state,
    select_parent,
    crossover,
    mutate,
};
// Define structs with serde and wasm_bindgen
#[wasm_bindgen]
extern "C" {
    // Import JavaScript Date object
    #[wasm_bindgen(typescript_type = "Date")]    pub type JsDate;
}



// Genetic Algorithm implementation with progress updates
#[wasm_bindgen]
pub fn get_schedule_recommendation(
    js_drivers: JsValue,
    js_vehicles: JsValue,
    js_orders: JsValue,
    js_update_function: &Function,
) -> Result<JsValue, JsValue> {
    // Deserialize input data from JsValue
    let drivers: Vec<Driver> = from_value(js_drivers)?;
    let vehicles: Vec<Vehicle> = from_value(js_vehicles)?;
    let orders: Vec<Order> = from_value(js_orders)?;

    // Initialize parameters
    let generations = 1000; // Use generations variable
    let population_size = 50;
    let mutation_rate = 0.1;
    let mandatory_break = ChronoDuration::minutes(30);
    let start_time = Instant::now();
    let max_duration = Duration::from_secs(10);

    // Map for order priorities
    let order_priority_map: HashMap<String, u32> = orders
        .iter()
        .map(|order| (order.id.clone(), order.priority.unwrap_or(1)))
        .collect();

    // Initialize population
    let mut population: Vec<SolverState> = Vec::new();
    for _ in 0..population_size {
        let state = initialize_random_state(
            &drivers,
            &vehicles,
            &orders,
            &order_priority_map,
            mandatory_break,
        );
        population.push(state);
    }

    let mut rng = rand::thread_rng();
    let mut generation = 0;

    // Use generations variable to control the loop
    while generation < generations && start_time.elapsed() < max_duration {
        // Evaluate fitness
        population.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());

        // Send progress update to JavaScript
        let progress = format!(
            "Generation {}: Best Score {:.2}",
            generation + 1,
            population[0].score
        );
        let _ = js_update_function.call1(&JsValue::NULL, &JsValue::from(progress));

        // Selection (elitism)
        let elite_count = (population_size as f64 * 0.1).ceil() as usize;
        let mut new_population: Vec<SolverState> = population[..elite_count].to_vec();

        // Crossover
        while new_population.len() < population_size {
            let parent1 = select_parent(&population);
            let parent2 = select_parent(&population);
            let child = crossover(
                parent1,
                parent2,
                &drivers,
                &vehicles,
                &orders,
                &order_priority_map,
                mandatory_break,
                &mut rng,
            );
            new_population.push(child);
        }

        // Mutation
        for individual in &mut new_population[elite_count..] {
            if rng.gen::<f64>() < mutation_rate {
                mutate(
                    individual,
                    &drivers,
                    &vehicles,
                    &orders,
                    &order_priority_map,
                    mandatory_break,
                    &mut rng,
                );
            }
        }

        population = new_population;
        generation += 1;
    }

    // Return the best solution
    population.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
    let best_assignments = &population[0].assignments;

    // Serialize the result to JsValue
    let response = SchedulingResponse {
        assignments: best_assignments.clone(),
    };
    let js_response = to_value(&response)?;

    Ok(js_response)
}