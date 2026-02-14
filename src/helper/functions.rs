use rand::Rng;
use regex::Regex;
use serde_json::Value;
use sha2::{Digest, Sha512};

pub fn control_body(list_needed: Vec<&str>, body: &Value) -> bool {
    for item in list_needed {
        if !body.get(item).is_some() {
            return false;
        }
    }
    true
}

// String manipulation part
pub fn sha512_string(input: &str) -> String {
    let mut hasher = Sha512::new();
    hasher.update(input);
    let result = hasher.finalize();
    let hash_string = result
        .iter()
        .map(|byte| format!("{:02x}", byte))
        .collect::<String>();
    hash_string
}

pub fn extract_string_from_obj_value(content: Option<&Value>) -> String {
    match content {
        Some(extract_value) => {
            match extract_value.as_str() {
                Some(v) => v.to_owned(), // Convert &str to String
                None => String::new(),
            }
        }
        None => String::new(),
    }
}

pub fn extract_int_from_obj_value(content: Option<&Value>) -> i32 {
    match content {
        Some(extract_value) => {
            match extract_value.as_i64() {
                Some(v) => v as i32, // Convert i64 to i32
                None => 0,
            }
        }
        None => 0,
    }
}

pub fn extract_bool_from_obj_value(content: Option<&Value>) -> bool {
    match content {
        Some(extract_value) => extract_value.as_bool().unwrap_or_else(|| false),
        None => false,
    }
}

pub fn crop_text(text: &str, length: usize) -> String {
    if text.len() > length {
        return format!("{}", &text[..length]);
    }
    return text.to_owned();
}

pub fn generate_random_number(length: usize) -> String {
    let mut rng = rand::thread_rng();
    let random_number: String = (0..length)
        .map(|_| rng.gen_range(0..=9).to_string())
        .collect();
    return random_number;
}

pub fn generate_random_string(length: usize) -> String {
    let mut rng = rand::thread_rng();
    let random_string: String = (0..length)
        .map(|_| rng.gen_range(0..=25) as u8 + b'a')
        .map(|b| b as char)
        .collect();
    return random_string;
}

pub fn generate_random_digit(length: usize) -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let mut random_digit = String::new();
    for _ in 0..length {
        random_digit.push_str(&rng.gen_range(0..10).to_string());
    }
    random_digit
}

/***
 *     ____  ____  ___  ____  _  _
 *    (  _ \( ___)/ __)( ___)( \/ )
 *     )   / )__)( (_-. )__)  )  (
 *    (_)\_)(____)\___/(____)(_/\_)
 */
pub fn is_valid_email(email: &str) -> bool {
    let re = Regex::new(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,10}$").unwrap();
    re.is_match(email)
}

pub fn is_valid_username(username: &str) -> bool {
    let re = Regex::new(r"^[a-zA-Z0-9._-]{3,25}$").unwrap();
    re.is_match(username)
}

pub fn is_valid_dpusername(dpusername: &str) -> bool {
    let re = Regex::new(r"^[a-zA-Z0-9\ ._-]{3,25}$").unwrap();
    re.is_match(dpusername)
}

pub fn is_valid_ref(dpusername: &str) -> bool {
    let re = Regex::new(r"^[a-zA-Z0-9#\ ._-]{3,25}$").unwrap();
    re.is_match(dpusername)
}

pub fn is_valid_legalname(dpusername: &str) -> bool {
    let re = Regex::new(r"^[\p{L}\p{M}\ -]{2,30}$").unwrap();
    re.is_match(dpusername)
}

pub fn is_valid_header(dpusername: &str) -> bool {
    let re = Regex::new(r"^[a-zA-Z0-9\ ._-]{1,50}$").unwrap();
    re.is_match(dpusername)
}

pub fn is_valid_header_value(dpusername: &str) -> bool {
    let re = Regex::new(r"^[a-zA-Z0-9\ ._-]{1,150}$").unwrap();
    re.is_match(dpusername)
}

pub fn is_valid_text(text: &str) -> bool {
    // Autorise lettres unicode, chiffres, ponctuation de base et certains symboles
    let re = Regex::new(r"^[a-zA-Z0-90-9<'/\:.,_\-\s]{3,2000}$").unwrap();
    re.is_match(text)
}

pub fn escape_string_for_sql(text: &str) -> String {
    text.replace("\\", "\\\\") // escape backslashes
        .replace("'", " ") // escape single quotes
}

pub fn is_valid_cryptocurrency_address(address: &str) -> bool {
    let re = Regex::new(r"^[a-zA-Z0-9]{20,50}$").unwrap();
    re.is_match(address)
}

pub fn is_valid_slug(text: &str) -> bool {
    let re = Regex::new(r"^[a-zA-Z0-9_-]{3,255}$").unwrap();
    re.is_match(text)
}

pub fn is_valid_small_text(text: &str) -> bool {
    // allow letters with accents, numbers, and specific special characters
    let re = Regex::new(r"^[\p{L}\p{M}\ -]{0,200}$").unwrap();
    re.is_match(text)
}

pub fn is_valid_name(text: &str) -> bool {
    // allow letters with accents, numbers, and specific special characters
    let re = Regex::new(r"^[\p{L}0-9'\ .,-]{1,25}$").unwrap();
    re.is_match(text)
}

pub fn is_uuid_v4(input: &str) -> bool {
    let re = Regex::new(r"^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$")
        .unwrap();
    re.is_match(input.to_lowercase().as_str())
}

pub fn is_valid_url(input: &str) -> bool {
    let re =
        Regex::new(r"^(https?://)?(?:[\w-]+\.)?[\w-]+\.[a-zA-Z]{2,}(?:/[\w/]{1,100})?$").unwrap();
    re.is_match(input)
}

pub fn is_valid_url_local(input: &str) -> bool {
    let re = Regex::new(r"^(?:/[\w\./]{1,100})$").unwrap();
    re.is_match(input)
}

pub fn is_valid_number(input: &str) -> bool {
    let re = Regex::new(r"^[0-9]{1,25}$").unwrap();
    re.is_match(input)
}

pub fn is_valid_sha512(input: &str) -> bool {
    let re = Regex::new(r"^[0-9a-f]{128}$").unwrap();
    re.is_match(input)
}

pub fn is_valid_domain(input: &str) -> bool {
    // valid domain name regex (including 1 subdomain)
    let re = Regex::new(r"^(?:[a-zA-Z0-9-]+\.)?[a-zA-Z0-9-]+\.[a-zA-Z]{2,10}$").unwrap();
    re.is_match(input)
}
