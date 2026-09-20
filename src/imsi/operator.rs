#[derive(Debug, Clone)]
pub struct Operator {
    pub country_code: &'static str,
    pub code: &'static str,
    pub name: &'static str,
    pub imsi_prefix: &'static str,
}

pub const OPERATORS: &[Operator] = &[
    Operator {
        country_code: "TUR",
        code: "TCELL",
        name: "Turkcell",
        imsi_prefix: "28601",
    },
    Operator {
        country_code: "TUR",
        code: "VF_TR",
        name: "Vodafone Turkiye",
        imsi_prefix: "28602",
    },
    Operator {
        country_code: "TUR",
        code: "TTELE",
        name: "Turk Telekom",
        imsi_prefix: "28604",
    },
    Operator {
        country_code: "USA",
        code: "ATT",
        name: "AT&T",
        imsi_prefix: "310410",
    },
    Operator {
        country_code: "USA",
        code: "VZ",
        name: "Verizon",
        imsi_prefix: "311480",
    },
    Operator {
        country_code: "UKR",
        code: "KS",
        name: "Kyivstar",
        imsi_prefix: "25501",
    },
];

/// Every operator belonging to `country_code`.
pub fn operators_for(country_code: &str) -> Vec<Operator> {
    OPERATORS
        .iter()
        .filter(|o| o.country_code == country_code)
        .cloned()
        .collect()
}

/// Look up an operator's IMSI prefix (MCC+MNC) by its code.
pub fn imsi_prefix_for(code: &str) -> Option<&'static str> {
    OPERATORS.iter().find(|o| o.code == code).map(|o| o.imsi_prefix)
}

/// Find the operator whose IMSI prefix (MCC+MNC) starts `pattern`, e.g. to
/// recover the operator from a saved checkpoint's composed pattern. Prefers
/// the longest matching prefix, since a shorter prefix can be a substring of
/// a longer, unrelated one.
pub fn operator_for_pattern(pattern: &str) -> Option<&'static Operator> {
    OPERATORS
        .iter()
        .filter(|o| pattern.starts_with(o.imsi_prefix))
        .max_by_key(|o| o.imsi_prefix.len())
}
