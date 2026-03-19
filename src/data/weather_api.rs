use serde::Deserialize;

use crate::error::NexClockError;

const OPEN_METEO_API_URL: &str = "https://api.open-meteo.com/v1/forecast";

/// Weather data returned to the UI.
#[derive(Debug, Clone)]
pub struct WeatherData {
    pub temperature: f64,
    pub unit: String,
    pub description: String,
    pub humidity: Option<u32>,
    pub precipitation_probability: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct OpenMeteoResponse {
    current: CurrentWeather,
    #[serde(default)]
    hourly: Option<HourlyWeather>,
}

#[derive(Debug, Deserialize)]
struct CurrentWeather {
    temperature_2m: f64,
    weather_code: u32,
    #[serde(default)]
    relative_humidity_2m: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct HourlyWeather {
    #[serde(default)]
    time: Vec<String>,
    #[serde(default)]
    precipitation_probability: Vec<u32>,
}

/// Fetches current weather from Open-Meteo.
pub async fn fetch_weather(
    latitude: f64,
    longitude: f64,
    unit: &str,
) -> Result<WeatherData, NexClockError> {
    let temp_unit = if unit == "fahrenheit" {
        "fahrenheit"
    } else {
        "celsius"
    };

    let url = format!(
        "{OPEN_METEO_API_URL}?latitude={latitude}&longitude={longitude}\
         &current=temperature_2m,weather_code,relative_humidity_2m\
         &hourly=precipitation_probability\
         &forecast_days=1\
         &temperature_unit={temp_unit}"
    );

    let response = reqwest::get(&url).await?;
    let data: OpenMeteoResponse = response.json().await?;

    let unit_symbol = if temp_unit == "fahrenheit" {
        "F"
    } else {
        "C"
    };

    // Extract current hour's precipitation probability from hourly data
    let precip_prob = extract_current_hour_precip(&data.hourly);

    Ok(WeatherData {
        temperature: data.current.temperature_2m,
        unit: unit_symbol.to_string(),
        description: wmo_code_to_description(data.current.weather_code),
        humidity: data.current.relative_humidity_2m,
        precipitation_probability: precip_prob,
    })
}

/// Finds the precipitation probability for the current hour from hourly data.
fn extract_current_hour_precip(hourly: &Option<HourlyWeather>) -> Option<u32> {
    let hourly = hourly.as_ref()?;
    let now = chrono::Local::now().format("%Y-%m-%dT%H:00").to_string();
    let idx = hourly.time.iter().position(|t| t == &now)?;
    hourly.precipitation_probability.get(idx).copied()
}

/// Maps WMO weather interpretation codes to human-readable descriptions.
fn wmo_code_to_description(code: u32) -> String {
    match code {
        0 => "Clear sky",
        1 => "Mainly clear",
        2 => "Partly cloudy",
        3 => "Overcast",
        45 | 48 => "Foggy",
        51 => "Light drizzle",
        53 => "Moderate drizzle",
        55 => "Dense drizzle",
        56 | 57 => "Freezing drizzle",
        61 => "Slight rain",
        63 => "Moderate rain",
        65 => "Heavy rain",
        66 | 67 => "Freezing rain",
        71 => "Slight snow",
        73 => "Moderate snow",
        75 => "Heavy snow",
        77 => "Snow grains",
        80 => "Slight rain showers",
        81 => "Moderate rain showers",
        82 => "Violent rain showers",
        85 => "Slight snow showers",
        86 => "Heavy snow showers",
        95 => "Thunderstorm",
        96 | 99 => "Thunderstorm with hail",
        _ => "Unknown",
    }
    .to_string()
}
