use indicatif::{ProgressBar, ProgressStyle};
use log::{info, warn};
use rayon::prelude::*;
use serde::Serialize;
use serde_json::to_string_pretty;
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::sync::{Arc, Mutex};
use clap::Parser;
use num_bigint::{BigUint, ToBigUint};
use num_traits::One;

#[derive(Parser)]
struct Args {
    #[clap(short, long)]
    file: std::path::PathBuf,
}

#[derive(Serialize)]
struct PrimeFactors {
    factors: HashMap<u64, u64>,
}

fn generate_primes_up_to(n: u64) -> Vec<u64> {
    let mut primes = vec![2];
    for num in 3..=n {
        if primes.iter().all(|prime| num % prime != 0) {
            primes.push(num);
        }
    }
    primes
}

fn compute_product(prime_powers: &HashMap<u64, u64>) -> BigUint {
    prime_powers
        .iter()
        .map(|(prime, power)| prime.to_biguint().unwrap().pow(*power as u32))
        .fold(BigUint::one(), |acc, x| acc * x)
}

fn log_guess(prime_powers: &HashMap<u64, u64>) {
    let guess: Vec<String> = prime_powers
        .iter()
        .map(|(prime, power)| format!("{}^{}", prime, power))
        .collect();
    info!("Current guess: {}", guess.join(" * "));
}

fn main() {
    env_logger::init();

    let args = Args::parse();

    let mut file = File::open(&args.file).expect("Failed to open the file");
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).expect("Failed to read the file");

    // Progress bar for file conversion
    let bar = ProgressBar::new(buffer.len() as u64);
    bar.set_style(
        ProgressStyle::default_bar()
            .template("{msg} {bar:40.cyan/blue} {pos}/{len}")
            .expect("Failed to set progress bar style"),
    );
    bar.set_message("Converting file to number");
    
    let number = BigUint::from_bytes_be(&buffer);
    bar.finish_with_message("File converted to number");

    info!("Number to factorize: {}", number);
    info!("Number of digits: {}", number.to_string().len());

    // Generate prime candidates up to sqrt(number)
    let sqrt_n = number.sqrt();
    let sqrt_u64 = sqrt_n.to_u64_digits()[0];
    let primes = generate_primes_up_to(sqrt_u64);

    info!(
        "Generated {} prime candidates up to sqrt({})",
        primes.len(),
        number
    );

    let mut prime_powers: HashMap<u64, u64> = primes.iter().map(|&prime| (prime, 0)).collect();
    let best_match = Arc::new(Mutex::new((BigUint::from(u64::MAX), prime_powers.clone())));

    // Progress bar for guessing prime powers
    let bar = ProgressBar::new(1000000);
    bar.set_style(
        ProgressStyle::default_bar()
            .template("{msg} {bar:40.cyan/blue} {pos}/{len}")
            .expect("Failed to set progress bar style"),
    );
    bar.set_message("Processing guesses");

    let found = Arc::new(Mutex::new(false));
    let total_iterations = 1000000;

    (0..total_iterations)
        .into_par_iter()
        .progress_with(bar.clone())
        .for_each(|iteration| {
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
                for prime in &primes {
                    if let Some(power) = local_prime_powers.get_mut(prime) {
                        *power += 1;
                        break;
                    }
                }
                log_guess(&local_prime_powers);

                let mut best_match = best_match.lock().unwrap();
                let current_distance = if &product > &number {
                    &product - &number
                } else {
                    &number - &product
                };
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
        println!(
            "Prime factors found: {}",
            to_string_pretty(&PrimeFactors {
                factors: best_match.1.clone()
            })
            .unwrap()
        );
    } else {
        println!(
            "Failed to find prime factors. Best match: {}",
            to_string_pretty(&PrimeFactors {
                factors: best_match.1.clone()
            })
            .unwrap()
        );
    }
}
