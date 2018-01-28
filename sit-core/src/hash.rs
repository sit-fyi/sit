//! SIT uses hashing for content-addressable entities (such as records)

/// Enumerates known hashing algorithm. Its content depends on features
/// enabled during build-time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HashingAlgorithm {
    #[cfg(feature = "blake2")]
    #[serde(rename = "blake2b")]
    /// [BLAKE2b] algorithm
    ///
    /// [BLAKE2b]: http://blake2.net/
    Blake2b {
        /// digest size
        size: usize,
    },
    #[cfg(feature = "sha-1")]
    #[serde(rename = "sha1")]
    /// [SHA-1] algorithm
    ///
    /// [SHA-1]: https://en.wikipedia.org/wiki/SHA-1
    SHA1,
}

impl Default for HashingAlgorithm {
    #[cfg(feature = "blake2")]
    fn default() -> Self {
        if cfg!(feature = "blake2") {
            // First preference is given to BLAKE2b. It's fast and has no known attacks
            HashingAlgorithm::Blake2b { size: 20 }
        } else if cfg!(feature = "sha-1") {
            // However, if it is turned off, use SHA-1 as a default hashing algorithm.
            // While there is a known attack against it, it is still a very popular
            // hashing algorithm
            HashingAlgorithm::SHA1
        } else {
            // Fail otherwise
            panic!("No hashing algorithms available. Make sure SIT is built with at least one hashing algorithm.")
        }
    }
}

#[cfg(feature = "blake2")]
use blake2;
#[cfg(feature = "sha-1")]
use sha1;


use digest::{FixedOutput, VariableOutput, Input};

/// Hasher is a unifying trait for different hashing algorithms
pub trait Hasher {
    /// Process stream input
    fn process(&mut self, input: &[u8]);
    /// Produce a hash
    fn result(self) -> Vec<u8>;
    /// Produce a hash out of a boxed `Hasher`
    fn result_box(self: Box<Self>) -> Vec<u8>;
}

/// Wraps any `FixedOutput` hashing algorithm
struct FixedOutputHasher<T: FixedOutput + Input>(T);

impl<T: FixedOutput + Input> Hasher for FixedOutputHasher<T> {
    fn process(&mut self, input: &[u8]) {
        self.0.process(input)
    }

    fn result(self) -> Vec<u8> {
        self.0.fixed_result().to_vec()
    }

    fn result_box(self: Box<Self>) -> Vec<u8> {
        self.0.fixed_result().to_vec()
    }
}

/// Wraps any `VariableOutput` hashing algorithm
struct VariableOutputHasher<T: VariableOutput + Input>(T);

impl<T: VariableOutput + Input> Hasher for VariableOutputHasher<T> {
    fn process(&mut self, input: &[u8]) {
        self.0.process(input)
    }

    fn result(self) -> Vec<u8> {
        let mut result = vec![0; self.0.output_size()];
        self.0.variable_result(&mut result).unwrap();
        result
    }

     fn result_box(self: Box<Self>) -> Vec<u8> {
        let mut result = vec![0; self.0.output_size()];
        self.0.variable_result(&mut result).unwrap();
        result
    }

}



impl HashingAlgorithm {

    /// Creates a boxed instance of [`Hasher`] for the algorithm
    ///
    /// [`Hasher`]: trait.Hasher.html
    pub fn hasher(&self) -> Box<Hasher> {
        match self {
            #[cfg(feature = "blake2")]
            &HashingAlgorithm::Blake2b { size } => Box::new(VariableOutputHasher(blake2::Blake2b::new(size).unwrap())),
            #[cfg(feature = "sha-1")]
            &HashingAlgorithm::SHA1 => Box::new(FixedOutputHasher(sha1::Sha1::default())),
        }
    }

}

#[cfg(test)]
mod tests {

    use super::*;

    #[cfg(feature = "blake2")]
    #[test]
    fn blake2() {
        let algo = HashingAlgorithm::Blake2b { size: 20 };
        let mut hasher = algo.hasher();
        hasher.process(b"test");
        hasher.process(b"that");
        // $ b2sum -l 160 <test file>
        // # returns
        // ef9ebcc4562d63642ef13cabe77a33a6994ead7f
        assert_eq!(hasher.result_box(), vec![239, 158, 188, 196, 86, 45, 99, 100, 46, 241, 60, 171, 231, 122, 51, 166, 153, 78, 173, 127]);
    }

    #[cfg(feature = "sha-1")]
    #[test]
    fn sha1() {
        let algo = HashingAlgorithm::SHA1;
        let mut hasher = algo.hasher();
        hasher.process(b"test");
        hasher.process(b"that");
        // $ sha1sum <test file>
        // # returns
        // 294863232e30c5580ee9410b7c35a2c6d3b6ceb3
        assert_eq!(hasher.result_box(), vec![41, 72, 99, 35, 46, 48, 197, 88, 14, 233, 65, 11, 124, 53, 162, 198, 211, 182, 206, 179]);
    }
}