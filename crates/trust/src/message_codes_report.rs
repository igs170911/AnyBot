pub const CODE_REPORT_STUCK_OVER_BUDGET: &str = "report-stuck-over-budget";
pub const CODE_REPORT_NEXT_RAISE_LIMIT: &str = "report-next-raise-limit";

#[must_use]
pub fn report_codes() -> Vec<&'static str> {
    vec![CODE_REPORT_STUCK_OVER_BUDGET, CODE_REPORT_NEXT_RAISE_LIMIT]
}
