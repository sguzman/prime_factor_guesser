use indicatif::{ParallelProgressIterator, ProgressBar, ProgressStyle};
use log::{info, warn};
use rayon::prelude::*;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

// Function to generate a vector of prime numbers up to sqrt(n)
fn generate_primes_up_to(n: u64) -> Vec<u64> {
    let mut primes = vec![2];
    for num in 3..=n {
        if primes.iter().all(|prime| num % prime != 0) {
            primes.push(num);
        }
    }
    primes
}

// Function to compute the product of the current guess
fn compute_product(prime_powers: &HashMap<u64, u64>) -> u64 {
    prime_powers.iter().map(|(prime, power)| prime.pow(*power as u32)).product()
}

// Function to log the current guess
fn log_guess(prime_powers: &HashMap<u64, u64>) {
    let guess: Vec<String> = prime_powers.iter().map(|(prime, power)| format!("{}^{}", prime, power)).collect();
    info!("Current guess: {}", guess.join(" * "));
}

fn main() {
    // Initialize the logger
    env_logger::init();

    // Example large number to factor
    let number: u64 = 123456789;

    // Generate prime candidates up to sqrt(number)
    let sqrt_n = (number as f64).sqrt() as u64;
    let primes = generate_primes_up_to(sqrt_n);

    info!("Generated {} prime candidates up to sqrt({})", primes.len(), number);

    // Initialize the guess with all powers set to 0
    let mut prime_powers: HashMap<u64, u64> = primes.iter().map(|&prime| (prime, 0)).collect();
    let best_match = Arc::new(Mutex::new((u64::MAX, prime_powers.clone())));

    // Progress bar
    let bar = ProgressBar::new(1000000);
    bar.set_style(
        ProgressStyle::default_bar()
            .template("{msg} {bar:40.cyan/blue} {pos}/{len}")
            .expect("Failed to set progress bar style"),
    );

    let found = Arc::new(Mutex::new(false));
    let total_iterations = 1000000; // Example number of iterations to try

    (0..total_iterations).into_par_iter().progress_with(bar.clone()).for_each(|iteration| {
        let mut local_prime_powers = prime_powers.clone();
        let product = compute_product(&local_prime_powers);

        {
            let mut found = found.lock().unwrap();
            if *found {
                return;
            }
        }

        if product == number {
            {
                let mut found = found.lock().unwrap();
                *found = true;
            }
            info!("Found prime factors: {:?}", local_prime_powers);
        } else {
            // Adjust the powers for the next guess
            for prime in &primes {
                if let Some(power) = local_prime_powers.get_mut(prime) {
                    *power += 1;
                    break;
                }
            }
            log_guess(&local_prime_powers);

            let mut best_match = best_match.lock().unwrap();
            let current_distance = (product as i64 - number as i64).abs() as u64;
            if current_distance < best_match.0 {
                best_match.0 = current_distance;
                best_match.1 = local_prime_powers.clone();
            }
        }

        if iteration % 1000 == 0 {
            warn!("Still running after {} iterations", iteration);
        }
    });

    bar.finish_with_message("Completed");

    let best_match = best_match.lock().unwrap();
    if *found.lock().unwrap() {
        println!("Prime factors found: {:?}", best_match.1);
    } else {
        println!("Failed to find prime factors. Best match: {:?}", best_match.1);
    }
}
