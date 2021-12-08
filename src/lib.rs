mod graph;
pub type Result<T> = std::result::Result<T, &'static str>;

pub fn run(upto: usize) -> Result<()> {
    let sieved = sieve_upto(upto)?;
    for i in sieved {
        println!("{}", i);
    }
    Ok(())
}

fn sieve_upto(upto: usize) -> Result<Vec<usize>> {
    let mut sieve : Vec<bool> = vec![true; upto-1 as usize];
    for i in 2..((upto as f64).sqrt() as usize) + 1 {
        if sieve[i-2] {
            let mut j = i * 2;
            while j <= upto {
                sieve[j-2] = false;
                j += i;
            }
        }
    }
    Ok(sieve.iter().enumerate().filter(|&(_, &is_p)| is_p).map(|(i, _)| (i+2)).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_sieve() {
        assert_eq!(sieve_upto(1), Ok(vec![]));
        assert_eq!(sieve_upto(2), Ok(vec![2]));
        assert_eq!(sieve_upto(3), Ok(vec![2, 3]));
        assert_eq!(sieve_upto(4), Ok(vec![2, 3]));
        assert_eq!(sieve_upto(5), Ok(vec![2, 3, 5]));
    }
}