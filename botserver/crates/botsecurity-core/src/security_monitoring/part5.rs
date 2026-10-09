use super::*;

impl GeoLocation {
    pub fn new() -> Self {
        Self {
            country: None,
            region: None,
            city: None,
            latitude: None,
            longitude: None,
            timezone: None,
        }
    }

    pub fn with_country(mut self, country: &str) -> Self {
        self.country = Some(country.to_string());
        self
    }

    pub fn with_city(mut self, city: &str) -> Self {
        self.city = Some(city.to_string());
        self
    }

    pub fn with_coordinates(mut self, lat: f64, lon: f64) -> Self {
        self.latitude = Some(lat);
        self.longitude = Some(lon);
        self
    }

    pub fn distance_km(&self, other: &GeoLocation) -> Option<f64> {
        match (self.latitude, self.longitude, other.latitude, other.longitude) {
            (Some(lat1), Some(lon1), Some(lat2), Some(lon2)) => {
                Some(haversine_distance(lat1, lon1, lat2, lon2))
            }
            _ => None,
        }
    }
}
impl Default for GeoLocation {
    fn default() -> Self {
        Self::new()
    }
}
