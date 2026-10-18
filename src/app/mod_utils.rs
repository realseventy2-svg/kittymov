pub fn lang_name(code: &str) -> &'static str {
    match code.to_lowercase().as_str() {
        "eng" => "English",
        "jpn" => "Japanese",
        "spa" => "Spanish",
        "fra" => "French",
        "deu" => "German",
        "ita" => "Italian",
        "por" => "Portuguese",
        "rus" => "Russian",
        "kor" => "Korean",
        "zho" => "Chinese",
        _ => "Custom",
    }
}

pub fn format_bytes(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else if bytes < 1024 * 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.2} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}

pub fn cycle_list(list: &[&str], current: &str, forward: bool) -> String {
    let current_lower = current.to_lowercase();
    let idx = list
        .iter()
        .position(|&x| x.to_lowercase() == current_lower)
        .unwrap_or(0);
    let next_idx = if forward {
        (idx + 1) % list.len()
    } else {
        (idx + list.len() - 1) % list.len()
    };
    list[next_idx].to_string()
}
