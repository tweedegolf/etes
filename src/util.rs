use rand::{Rng, distributions::Alphanumeric};
use sha2::Digest;
use tokio::net::TcpListener;

// Get a free port number on localhost
pub async fn get_free_port() -> Option<u16> {
    let listener: TcpListener = TcpListener::bind("127.0.0.1:0").await.ok()?;

    Some(listener.local_addr().ok()?.port())
}

// Check if a character is a valid character for a name
pub fn is_normal_char(c: char) -> bool {
    matches!(c, 'a'..='z' | 'A'..='Z' | '0'..='9' | '-')
}

// Check if a string is a valid name
pub fn is_valid_name(name: &str) -> bool {
    name.chars().all(is_normal_char) && !name.is_empty() && name.len() < 128
}

// Check if a string is a valid git commit hash
pub fn is_valid_hash(hash: &str) -> bool {
    hash.len() == 40 && hash.chars().all(|c| c.is_ascii_hexdigit())
}

// Generate a random string
pub fn random_string() -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(24)
        .map(char::from)
        .collect()
}

// Hash a string using SHA-256
pub fn sha256(input: &str) -> String {
    let mut hasher = sha2::Sha256::new();
    hasher.update(input.as_bytes());

    hasher
        .finalize()
        .iter()
        .map(|b| format!("{:02x}", b).to_string())
        .collect::<Vec<String>>()
        .join("")
}

pub fn sha512(input: &str) -> [u8; 64] {
    let mut hasher = sha2::Sha512::new();
    hasher.update(input.as_bytes());

    hasher.finalize().into()
}

// Get a random name from a list of words
pub fn get_random_name(words: &[String]) -> String {
    let mut rng = rand::thread_rng();
    let mut name = Vec::new();

    while name.len() < 3 {
        let word = words[rng.gen_range(0..words.len())].as_str();

        if !name.contains(&word) {
            name.push(word);
        }
    }

    name.join("-")
}

#[cfg(test)]
mod test {
    #[test]
    fn test_is_valid_name() {
        assert!(!super::is_valid_name(""));
        assert!(super::is_valid_name("test"));
        assert!(super::is_valid_name("test-123"));
        assert!(super::is_valid_name("test-123-abc"));
        assert!(super::is_valid_name("test-123-abc-"));
        assert!(!super::is_valid_name("test-123-abc-!"));
        assert!(!super::is_valid_name("test-123-abc-!@#$%^&*()_+|"));
        assert!(super::is_valid_name(
            "testtesttesttesttesttesttesttesttesttesttesttesttesttesttestttesttesttesttesttesttesttesttest"
        ));
        assert!(!super::is_valid_name(
            "testtesttesttesttesttesttesttesttesttesttesttesttesttesttestttesttesttesttesttesttesttesttesttesttesttesttesttesttesttesttesttesttest"
        ));
    }

    #[test]
    fn test_is_valid_hash() {
        assert!(!super::is_valid_hash("test"));
        assert!(super::is_valid_hash(
            "4f5d3be66fb5324eda7c05c9d95b777f057d25f9"
        ));
    }

    #[test]
    fn test_sha256() {
        assert_eq!(
            super::sha256("test"),
            "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08"
        );
    }

    #[test]
    fn test_get_random_name() {
        let words = vec![
            "test".to_string(),
            "random".to_string(),
            "words".to_string(),
        ];

        let name = super::get_random_name(&words);
        assert!(
            name == "test-random-words"
                || name == "test-words-random"
                || name == "random-test-words"
                || name == "random-words-test"
                || name == "words-random-test"
                || name == "words-test-random"
        );
    }
}
