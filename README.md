## Problem Overview
We are solving a scheduling problem where we need to assign orders to drivers and vehicles while respecting various constraints:

- Time Constraints: Orders have start and end times.
- Resource Constraints: Drivers and vehicles have schedules and can't be in two places at once.
- Capacity Constraints: Vehicles have weight and volume limits.
- Mandatory Breaks: Drivers and vehicles require mandatory breaks between assignments.
- Preferences and Tags: Drivers may have vehicle preferences, and orders may have tags that need to match vehicle tags.

Our objective is to maximize the total score, which is based on order priorities and driver preferences.

## Algorithm Overview
We use a **Genetic Algorithm (GA)** to find an approximate solution within a given time limit (10 seconds). The GA evolves a population of solutions over several generations, using selection, crossover, and mutation to improve the solutions.

## Data Structures
### Order

```rust
pub struct Order {
    pub id: String,
    pub start_time: NaiveDateTime,
    pub end_time: NaiveDateTime,
    pub priority: Option<u32>,
    pub tags: Option<Vec<String>>,
    pub weight: f64,
    pub volume: Option<f64>,
}
```



## Helper Functions
orders_overlap: Checks if two orders overlap in time.
insufficient_break: Checks if there is insufficient break time between two orders.
- is_driver_on_break: Checks if a driver is on a break at a given time.
- can_assign_driver: Determines if a driver can be assigned an order.
- can_assign_vehicle: Determines if a vehicle can be assigned an order.
- can_assign: Checks if both the driver and vehicle can be assigned an order.

## Genetic Algorithm Components
- Initialization: Create an initial population of random feasible solutions.
- Fitness Evaluation: Calculate the score of each solution.
- Selection: Select parent solutions based on their fitness.
- Crossover: Combine parents to create offspring solutions.
- Mutation: Introduce random changes to offspring.
- Replacement: Form a new population from the offspring.
- Termination: Repeat steps 2-6 until a stopping condition is met (time limit or number of generations).