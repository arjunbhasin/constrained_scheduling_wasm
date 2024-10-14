// src/lib.rs
mod solver;
use chrono::Duration as ChronoDuration;
use console_error_panic_hook;
use js_sys::{Date, Function};
use rand::prelude::*;
use serde_wasm_bindgen::{from_value, to_value};
use solver::{
    crossover, initialize_random_state, mutate, select_parent, Driver, Order, SchedulingResponse,
    SolverState, Vehicle,
};
use std::cmp::Ordering;
use std::collections::HashMap;
use wasm_bindgen::prelude::*;
// Define structs with serde and wasm_bindgen
#[wasm_bindgen]
extern "C" {
    // Import JavaScript Date object
    #[wasm_bindgen(typescript_type = "Date")]
    pub type JsDate;
}

#[wasm_bindgen(start)]
pub fn main_js() {
    // Set the panic hook
    console_error_panic_hook::set_once();
}

// Genetic Algorithm implementation with progress updates and termination criterion
#[wasm_bindgen]
pub fn get_schedule_recommendation(
    js_drivers: JsValue,
    js_vehicles: JsValue,
    js_orders: JsValue,
    js_update_function: &Function,
) -> Result<JsValue, JsValue> {
    // Deserialize input data from JsValue
    let drivers: Vec<Driver> = from_value(js_drivers)
        .map_err(|e| JsValue::from_str(&format!("Failed to deserialize drivers: {}", e)))?;
    let vehicles: Vec<Vehicle> = from_value(js_vehicles)
        .map_err(|e| JsValue::from_str(&format!("Failed to deserialize vehicles: {}", e)))?;
    let orders: Vec<Order> = from_value(js_orders)
        .map_err(|e| JsValue::from_str(&format!("Failed to deserialize orders: {}", e)))?;

    // Initialize parameters
    let generations = 1000; // Maximum number of generations
    let population_size = 50;
    let mutation_rate = 0.1;
    let mandatory_break = ChronoDuration::minutes(30);
    let start_time = Date::now(); // Milliseconds since epoch as f64
    let max_duration = 10_000.0; // 10 seconds in milliseconds

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

    // Initialize variables for termination criterion
    let mut best_score = population[0].score;
    let mut generations_without_improvement = 0;
    let max_generations_without_improvement = 50; // Adjust as needed

    // Use generations variable to control the loop
    while (Date::now() - start_time) < max_duration && generation < generations {
        // Evaluate fitness
        population.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(Ordering::Equal));
        let current_best_score = population[0].score;

        // Send progress update to JavaScript
        let progress = format!(
            "Generation {}: Best Score {:.2}",
            generation + 1,
            current_best_score
        );
        let _ = js_update_function.call1(&JsValue::NULL, &JsValue::from(progress));

        // Check for improvement
        if current_best_score > best_score {
            best_score = current_best_score;
            generations_without_improvement = 0;
        } else {
            generations_without_improvement += 1;
        }

        // Terminate if no improvement over threshold
        if generations_without_improvement >= max_generations_without_improvement {
            let termination_message = format!(
                "No improvement over {} generations, terminating.",
                max_generations_without_improvement
            );
            let _ = js_update_function.call1(&JsValue::NULL, &JsValue::from(termination_message));
            break;
        }

        // Selection (elitism)
        let elite_count = (population_size as f64 * 0.1).ceil() as usize;
        let mut new_population: Vec<SolverState> = population[..elite_count].to_vec();

        // Crossover
        while new_population.len() < population_size {
            let parent1 = select_parent(&population);
            let parent2 = select_parent(&population);
            let mut child = crossover(
                parent1,
                parent2,
                &drivers,
                &vehicles,
                &orders,
                &order_priority_map,
                mandatory_break,
                &mut rng,
            );

            // Mutation
            if rng.gen::<f64>() < mutation_rate {
                mutate(
                    &mut child,
                    &drivers,
                    &vehicles,
                    &orders,
                    &order_priority_map,
                    mandatory_break,
                    &mut rng,
                );
            }

            // Recalculate the child's score after mutation
            child.score = child.calculate_score(&order_priority_map, &drivers);
            new_population.push(child);
        }

        population = new_population;
        generation += 1;
    }

    // Return the best solution
    population.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(Ordering::Equal));
    let best_assignments = &population[0].assignments;

    // Serialize the result to JsValue
    let response = SchedulingResponse {
        assignments: best_assignments.clone(),
    };
    let js_response = to_value(&response)?;

    Ok(js_response)
}
