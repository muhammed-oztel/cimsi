#[derive(Debug, Clone)]
pub struct Country {
    pub code: &'static str,
    pub name: &'static str,
}

pub const COUNTRIES: &[Country] = &[
    Country {
        code: "TUR",
        name: "Turkiye",
    },
    Country {
        code: "USA",
        name: "United States",
    },
    Country {
        code: "UKR",
        name: "Ukraine",
    },
];
