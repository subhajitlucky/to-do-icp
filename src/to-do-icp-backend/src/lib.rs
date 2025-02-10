use ic_cdk::{update, query};
use ic_cdk::api::time;
use candid::{CandidType, Principal};
use std::collections::HashMap;
use serde::{Serialize, Deserialize};

// Canister state with stable storage
thread_local! {
    static TASKS: std::cell::RefCell<HashMap<u64, Task>> = std::cell::RefCell::new(HashMap::new());
    static NEXT_ID: std::cell::RefCell<u64> = std::cell::RefCell::new(0);
}

// Add RepeatCycle enum for task repetition
#[derive(Clone, Debug, Serialize, Deserialize, CandidType)]
enum RepeatCycle {
    Daily,
    Weekly,
    Monthly,
    Yearly,
}

// Updated Task struct
#[derive(Clone, Debug, Default, Serialize, Deserialize, CandidType)]
struct Task {
    id: u64,
    title: String,
    is_completed: bool,
    is_important: bool,
    due_date: Option<u64>,      // Timestamp for due date
    reminder: Option<u64>,      // Timestamp for reminder
    repeat: Option<RepeatCycle>, // Repeat frequency
    assigned_to: Option<Principal>,
}

// Input for adding a task (some fields optional)
#[derive(Deserialize, CandidType)]
struct TaskInput {
    title: String,              // Mandatory
    is_important: Option<bool>, // Optional
    due_date: Option<u64>,      // Optional
    reminder: Option<u64>,      // Optional
    repeat: Option<RepeatCycle>, // Optional
    assigned_to: Option<Principal>, // Optional
}

// Add a new task (only title is required)
#[update]
fn add_task(input: TaskInput) -> u64 {
    let id = NEXT_ID.with(|next_id| {
        let mut next_id = next_id.borrow_mut();
        let id = *next_id;
        *next_id += 1;
        id
    });

    let task = Task {
        id,
        title: input.title,
        is_completed: false, // Default to incomplete
        is_important: input.is_important.unwrap_or(false),
        due_date: input.due_date,
        reminder: input.reminder,
        repeat: input.repeat,
        assigned_to: input.assigned_to,
    };

    TASKS.with(|tasks| {
        tasks.borrow_mut().insert(id, task);
    });

    id
}

// Input for updating a task (all fields optional)
#[derive(Deserialize, CandidType)]
struct UpdateTaskInput {
    title: Option<String>,
    is_completed: Option<bool>,
    is_important: Option<bool>,
    due_date: Option<Option<u64>>,     // Can set to `Some(None)` to clear
    reminder: Option<Option<u64>>,
    repeat: Option<Option<RepeatCycle>>,
    assigned_to: Option<Option<Principal>>,
}

// Update an existing task by ID
#[update]
fn update_task(id: u64, input: UpdateTaskInput) -> Result<(), String> {
    TASKS.with(|tasks| {
        let mut tasks = tasks.borrow_mut();
        let task = tasks.get_mut(&id).ok_or("Task not found")?;

        // Update fields if provided
        if let Some(title) = input.title {
            task.title = title;
        }
        if let Some(is_completed) = input.is_completed {
            task.is_completed = is_completed;
        }
        if let Some(is_important) = input.is_important {
            task.is_important = is_important;
        }
        if let Some(due_date) = input.due_date {
            task.due_date = due_date;
        }
        if let Some(reminder) = input.reminder {
            task.reminder = reminder;
        }
        if let Some(repeat) = input.repeat {
            task.repeat = repeat;
        }
        if let Some(assigned_to) = input.assigned_to {
            task.assigned_to = assigned_to;
        }

        Ok(())
    })
}

#[query]
fn get_all_tasks() -> Vec<Task> {
    TASKS.with(|tasks| {
        tasks.borrow().values().cloned().collect()
    })
}

// Returns all tasks that are NOT completed
#[query]
fn get_active_tasks() -> Vec<Task> {
    TASKS.with(|tasks| {
        tasks.borrow()
            .values()
            .filter(|task| !task.is_completed)
            .cloned()
            .collect()
    })
}

// Returns all completed tasks
#[query]
fn get_completed_tasks() -> Vec<Task> {
    TASKS.with(|tasks| {
        tasks.borrow()
            .values()
            .filter(|task| task.is_completed)
            .cloned()
            .collect()
    })
}

//get today tasks
#[query]
fn get_today_tasks() -> Vec<Task> {
    // Get current UTC time in seconds
    let now_seconds = ic_cdk::api::time() / 1_000_000_000; // Convert nanoseconds to seconds

    // Calculate start and end of today (UTC)
    let start_of_day = now_seconds - (now_seconds % 86400); // 86400 seconds = 1 day
    let end_of_day = start_of_day + 86399; // 23:59:59

    TASKS.with(|tasks| {
        tasks.borrow()
            .values()
            .filter(|task| {
                // Check if task.due_date falls within today
                task.due_date.map_or(false, |due_date| {
                    due_date >= start_of_day && due_date <= end_of_day
                })
            })
            .cloned()
            .collect()
    })
}

//getting important tasks
#[query]
fn get_important_tasks() -> Vec<Task> {
    TASKS.with(|tasks| {
        tasks.borrow()
            .values()
            .filter(|task| task.is_important)
            .cloned()
            .collect()
    })
}

//get planned tasks of future
#[query]
fn get_planned_tasks() -> Vec<Task> {
    let now_seconds = ic_cdk::api::time() / 1_000_000_000; // Current UTC time in seconds

    TASKS.with(|tasks| {
        tasks.borrow()
            .values()
            .filter(|task| {
                // Include tasks with a due date in the future
                task.due_date.map_or(false, |due_date| due_date > now_seconds)
            })
            .cloned()
            .collect()
    })
}

//get tasks assigned to the caller
#[query]
fn get_assigned_tasks() -> Vec<Task> {
    let caller_principal = ic_cdk::caller(); // Get the principal of the current user

    TASKS.with(|tasks| {
        tasks.borrow()
            .values()
            .filter(|task| {
                // Check if task is assigned to the caller
                task.assigned_to.map_or(false, |assigned_principal| {
                    assigned_principal == caller_principal
                })
            })
            .cloned()
            .collect()
    })
}

// Function to count today's tasks
#[query]
fn count_today_tasks() -> u64 {
    // Reuse date logic from get_today_tasks()
    let now_seconds = ic_cdk::api::time() / 1_000_000_000;
    let start_of_day = now_seconds - (now_seconds % 86400);
    let end_of_day = start_of_day + 86399;

    TASKS.with(|tasks| {
        tasks.borrow()
            .values()
            .filter(|task| {
                task.due_date.map_or(false, |due_date| {
                    due_date >= start_of_day && due_date <= end_of_day
                })
            })
            .count() as u64 // Return count as u64
    })
}


// Function to delete a task by ID
#[update]
fn delete_task(id: u64) -> Result<(), String> {
    TASKS.with(|tasks| {
        let mut tasks = tasks.borrow_mut();
        if tasks.remove(&id).is_none() {
            Err("Task not found".to_string())
        } else {
            Ok(())
        }
    })
}