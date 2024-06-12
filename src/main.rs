use indicatif::{ParallelProgressIterator, ProgressBar, ProgressStyle};
use log::{debug, info, warn, LevelFilter};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::to_string_pretty;
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use clap::Parser;
use num_bigint::{BigUint, ToBigUint};
use num_traits::One;

#[derive(Parser, Debug)]
struct Args {
    #[clap(short, long)]
    file: PathBuf,
    #[clap(short, long)]
    cache: Option<PathBuf>,
}

#[derive(Serialize, Deserialize)]
struct PrimeFactors {
    factors: HashMap<u64, u64>,
}

fn generate_primes_up_to(n: u64, cache_file: Option<&PathBuf>) -> Vec<u64> {
    if let Some(cache_file) = cache_file {
        if let Ok(cached_primes) = read_primes_from_cache(cache_file) {
            return cached_primes;
        }
    }

    let bar = ProgressBar::new(n);
    bar.set_style(
        ProgressStyle::default_bar()
            .template("{msg} {bar:40.cyan/blue} {pos}/{len} {eta}")
            .expect("Failed to set progress bar style")
            .progress_chars("#>-"),
    );
    bar.set_message("Generating primes");

    let primes: Vec<u64> = (2..=n)
        .into_par_iter()
        .progress_with(bar.clone())
        .filter(|num| (2..(*num as f64).sqrt() as u64 + 1).all(|i| num % i != 0))
        .collect();

    bar.finish_with_message("Prime generation completed");

    if let Some(cache_file) = cache_file {
        write_primes_to_cache(cache_file, &primes).expect("Failed to write primes to cache");
    }

    primes
}

fn read_primes_from_cache(cache_file: &PathBuf) -> Result<Vec<u64>, std::io::Error> {
    let mut file = File::open(cache_file)?;
    let mut buffer = String::new();
    file.read_to_string(&mut buffer)?;
    let primes: Vec<u64> = serde_json::from_str(&buffer)?;
    Ok(primes)
}

fn write_primes_to_cache(cache_file: &PathBuf, primes: &[u64]) -> Result<(), std::io::Error> {
    let mut file = File::create(cache_file)?;
    let data = serde_json::to_string(primes)?;
    file.write_all(data.as_bytes())?;
    Ok(())
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
    debug!("Current guess: {}", guess.join(" * "));
}

fn main() {
    // Initialize logging
    init_logging();

    let args = Args::parse();
    debug!("Parsed arguments: {:?}", args);

    let mut file = File::open(&args.file).expect("Failed to open the file");
    debug!("Opened file: {:?}", args.file);

    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).expect("Failed to read the file");
    debug!("Read file content. Buffer length: {}", buffer.len());

    let number = BigUint::from_bytes_be(&buffer);
    info!("Number to factorize: {}", number);
    info!("Number of digits: {}", number.to_string().len());

    let sqrt_n = number.sqrt();
    let sqrt_u64 = sqrt_n.to_u64_digits()[0];
    let primes = generate_primes_up_to(sqrt_u64, args.cache.as_ref());
    debug!("Generated primes up to sqrt(n): {:?}", primes);

    info!(
        "Generated {} prime candidates up to sqrt({})",
        primes.len(),
        number
    );

    let mut prime_powers: HashMap<u64, u64> = primes.iter().map(|&prime| (prime, 0)).collect();
    let best_match = Arc::new(Mutex::new((BigUint::from(u64::MAX), prime_powers.clone())));

    let total_iterations = 1000000;
    let bar = ProgressBar::new(total_iterations as u64);
    bar.set_style(
        ProgressStyle::default_bar()
            .template("{msg} {bar:40.cyan/blue} {pos}/{len}")
            .expect("Failed to set progress bar style")
            .progress_chars("#>-"),
    );
    bar.set_message("Processing guesses");

    let found = Arc::new(Mutex::new(false));

    debug!("Starting parallel iteration with progress bar.");
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

fn init_logging() {
    env_logger::builder()
        .filter_level(LevelFilter::Debug)
        .init();
}
