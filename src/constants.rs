#[cfg(debug_assertions)]
pub const API_URL: &'static str = "http://localhost:8080";
#[cfg(not(debug_assertions))]
pub const API_URL: &'static str = "https://api.lilith.rip";
