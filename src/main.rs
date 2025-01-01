/// Controls the verbosity of output in calculate_take_home
#[derive(Default)]
struct LogConfig {
    show_deductions: bool,
    show_tax_breakdown: bool,
    show_insurance_breakdown: bool,
    show_summary: bool, // For the percentage breakdowns and final amounts
}

impl LogConfig {
    // fn all() -> Self {
    //     LogConfig {
    //         show_deductions: true,
    //         show_tax_breakdown: true,
    //         show_insurance_breakdown: true,
    //         show_summary: true,
    //     }
    // }

    // fn summary_only() -> Self {
    //     LogConfig {
    //         show_deductions: false,
    //         show_tax_breakdown: false,
    //         show_insurance_breakdown: false,
    //         show_summary: true,
    //     }
    // }

    fn none() -> Self {
        LogConfig {
            show_deductions: false,
            show_tax_breakdown: false,
            show_insurance_breakdown: false,
            show_summary: false,
        }
    }
}

struct IncomeAnalysis {
    annual_income: i64,
    monthly_costs: Option<MonthlyCosts>,
    log_config: Option<LogConfig>,
}

struct MonthlyCosts {
    fixed_costs: i64,      // Fixed monthly costs in yen
    percentage_costs: f64, // Variable costs as percentage of take-home
}

impl MonthlyCosts {
    fn calculate_total(&self, monthly_take_home: i64) -> i64 {
        let variable_costs = (monthly_take_home as f64 * self.percentage_costs / 100.0) as i64;
        self.fixed_costs + variable_costs
    }
}

struct SavingsTimeframe {
    months: i64,
    label: String, // e.g., "1 Year", "2 Years", etc.
}

/// Tax brackets for national income tax calculation.
/// Each tuple contains (income_threshold, rate_in_basis_points, deduction_amount)
///
/// The brackets are structured with exclusive upper bounds, meaning:
/// - ¥0 to ¥1,949,999: 5% with no deduction
/// - ¥1,950,000 to ¥3,299,999: 10% with ¥97,500 deduction
/// - ¥3,300,000 to ¥6,949,999: 20% with ¥427,500 deduction
/// - ¥6,950,000 to ¥8,999,999: 23% with ¥636,000 deduction
/// - ¥9,000,000 to ¥17,999,999: 33% with ¥1,536,000 deduction
/// - ¥18,000,000 to ¥39,999,999: 40% with ¥2,796,000 deduction
/// - ¥40,000,000 and above: 45% with ¥4,796,000 deduction
const TAX_BRACKETS: [(i64, i64); 7] = [
    (1_949_999, 500),
    (3_299_999, 1000),
    (6_949_999, 2000),
    (8_999_999, 2300),
    (17_999_999, 3300),
    (39_999_999, 4000),
    (i64::MAX, 4500),
];

/// Basic deduction brackets for earned income calculation.
/// Each tuple contains (income_threshold, rate_in_basis_points, adjustment_amount)
///
/// The brackets are structured with exclusive upper bounds, meaning:
/// - ¥0 to ¥1,624,999: Flat ¥550,000
/// - ¥1,625,000 to ¥1,799,999: 40% of income minus ¥100,000
/// - ¥1,800,000 to ¥3,599,999: 30% of income plus ¥80,000
/// - ¥3,600,000 to ¥6,599,999: 20% of income plus ¥440,000
/// - ¥6,600,000 to ¥8,499,999: 10% of income plus ¥1,100,000
/// - ¥8,500,000 and above: Flat ¥1,950,000
const BASIC_DEDUCTION_BRACKETS: [(i64, i64, i64); 6] = [
    (1_624_999, 0, 550_000),      // Up to 1,624,999
    (1_799_999, 4000, -100_000),  // 1,625,000 to 1,799,999
    (3_599_999, 3000, 80_000),    // 1,800,000 to 3,599,999
    (6_599_999, 2000, 440_000),   // 3,600,000 to 6,599,999
    (8_499_999, 1000, 1_100_000), // 6,600,000 to 8,499,999
    (i64::MAX, 0, 1_950_000),     // 8,500,000 and above
];

/// Personal exemption brackets for both national and local tax calculations.
/// Each tuple contains (income_threshold, national_exemption, local_exemption)
///
/// The brackets are structured with exclusive upper bounds, meaning:
/// - ¥0 to ¥23,999,999: National ¥480,000, Local ¥430,000
/// - ¥24,000,000 to ¥24,499,999: National ¥320,000, Local ¥290,000
/// - ¥24,500,000 to ¥24,999,999: National ¥160,000, Local ¥150,000
/// - ¥25,000,000 and above: No exemptions
const PERSONAL_EXEMPTION_BRACKETS: [(i64, i64, i64); 4] = [
    (23_999_999, 480_000, 430_000), // Up to 23,999,999
    (24_499_999, 320_000, 290_000), // 24,000,000 to 24,499,999
    (24_999_999, 160_000, 150_000), // 24,500,000 to 24,999,999
    (i64::MAX, 0, 0),               // 25,000,000 and above
];

const PREFECTURAL_TAX_RATE: i64 = 400;

const MUNICIPAL_TAX_RATE: i64 = 600;

const EQUALISATION_PER_CAPITA_TAX: i64 = 5_000;

const NATIONAL_FIXED_AMOUNT_TAX_REDUCTION: i64 = 30_000;
const LOCAL_FIXED_AMOUNT_TAX_REDUCTION: i64 = 10_000;

/// Constants for health insurance calculations in Setagaya
const BASIC_HEALTH_INSURANCE_RATE: i64 = 869; // 8.69%
const SUPPORT_HEALTH_INSURANCE_RATE: i64 = 280; // 2.80%
const DEPENDENT_BASIC_AMOUNT: i64 = 49_100;
const DEPENDENT_SUPPORT_AMOUNT: i64 = 16_500;

const ANNUAL_BASIC_CAP: i64 = 650_000;
const ANNUAL_SUPPORT_CAP: i64 = 240_000;

const UNEMPLOYMENT_INSURANCE_RATE: i64 = 60; // 0.6%

const PENSION_INSURANCE_RATE: i64 = 915; // 9.15%
const PENSION_INSURANCE_CAP: i64 = 713_700;

fn format_yen(amount: i64) -> String {
    let num_str = amount.to_string();
    let len = num_str.len();
    let mut result = String::with_capacity(len + (len - 1) / 3);

    for (i, c) in num_str.chars().enumerate() {
        if i > 0 && (len - i) % 3 == 0 {
            result.push(',');
        }
        result.push(c);
    }

    format!("¥{}", result)
}

/// Calculates the earned income deduction based on annual income.
///
/// The deduction is calculated using different formulas depending on the income bracket.
/// The calculation includes either a flat rate or a percentage of income plus an adjustment,
/// depending on which bracket the income falls into.
///
/// # Arguments
/// * `annual_income` - Gross annual income in yen
///
/// # Returns
/// The basic deduction amount in yen
fn get_basic_deduction(annual_income: i64) -> i64 {
    // Find the applicable bracket
    let (_, rate, adjustment) = BASIC_DEDUCTION_BRACKETS
        .iter()
        .find(|&&(bracket, _, _)| annual_income <= bracket)
        .copied()
        .unwrap_or((i64::MAX, 0, 1_950_000));

    // For the first bracket (flat rate 550,000)
    if rate == 0 {
        return adjustment;
    }

    // Calculate deduction using the formula: (rate * income / 10000) + adjustment
    let deduction = (annual_income * rate) / 10_000 + adjustment;

    deduction
}

/// Calculates personal exemptions for both national and local tax.
///
/// The exemption amounts decrease as income increases, with both national
/// and local exemptions reducing at specific income thresholds.
///
/// # Arguments
/// * `annual_income` - Gross annual income in yen
///
/// # Returns
/// A tuple containing (national_exemption, local_exemption) in yen
fn get_personal_exemptions(annual_income: i64) -> (i64, i64) {
    PERSONAL_EXEMPTION_BRACKETS
        .iter()
        .find(|&&(bracket, _, _)| annual_income <= bracket)
        .map(|&(_, national, local)| (national, local))
        .unwrap_or((0, 0))
}

/// Calculates national income tax based on annual income.
///
/// The calculation uses a progressive tax rate system where different
/// portions of income are taxed at different rates. The final tax amount
/// is reduced by a bracket-specific deduction.
///
/// # Arguments
/// * `annual_income` - Gross annual income in yen
///
/// # Returns
/// The calculated income tax amount in yen
fn get_income_tax(annual_income: i64) -> i64 {
    let remaining_income = annual_income;
    let mut total_tax = 0;
    let mut bracket_lower_bound = 0;

    for &(bracket_upper_bound, rate) in &TAX_BRACKETS {
        if remaining_income > bracket_upper_bound {
            // Calculate tax for the entire bracket
            let amount_in_bracket = bracket_upper_bound - bracket_lower_bound;
            let tax_for_bracket = amount_in_bracket * rate;
            total_tax += tax_for_bracket;
            bracket_lower_bound = bracket_upper_bound;
            continue;
        }

        // We're in the final applicable bracket
        let amount_in_bracket = remaining_income - bracket_lower_bound;
        let tax_for_bracket = amount_in_bracket * rate;
        total_tax += tax_for_bracket;
        break;
    }

    // Divide by 10,000 to convert basis points to yen
    total_tax / 10_000
}

/// Calculates prefectural tax based on annual income
fn get_prefectural_tax(annual_income: i64) -> i64 {
    annual_income * PREFECTURAL_TAX_RATE / 10000
}

/// Calculates municipal tax based on annual income
fn get_municipal_tax(annual_income: i64) -> i64 {
    annual_income * MUNICIPAL_TAX_RATE / 10000
}

/// Calculates health insurance premium based on income and number of dependents
///
/// # Arguments
/// * `assessed_premium` - Annual income used for calculation
/// * `num_dependents` - Number of dependents
///
/// # Returns
/// The total health insurance premium in yen
fn get_health_insurance(assessed_premium: i64, num_dependents: i64) -> i64 {
    // Calculate basic portion
    let basic_premium = (assessed_premium * BASIC_HEALTH_INSURANCE_RATE) / 10_000;
    let basic_dependent = num_dependents * DEPENDENT_BASIC_AMOUNT;
    let basic_portion = basic_premium + basic_dependent;

    // Calculate support portion
    let support_premium = (assessed_premium * SUPPORT_HEALTH_INSURANCE_RATE) / 10_000;
    let support_dependent = num_dependents * DEPENDENT_SUPPORT_AMOUNT;
    let support_portion = support_premium + support_dependent;

    // Total premium
    basic_portion.min(ANNUAL_BASIC_CAP) + support_portion.min(ANNUAL_SUPPORT_CAP)
}

/// Calculates unemployment insurance premium based on income
///
/// # Arguments
/// * `assessed_premium` - Annual income used for calculation
///
/// # Returns
/// The unemployment insurance premium in yen
fn get_unemployment_insurance(assessed_premium: i64) -> i64 {
    (assessed_premium * UNEMPLOYMENT_INSURANCE_RATE) / 10_000
}

/// Calculates pension insurance premium based on income
///
/// # Arguments
/// * `assessed_premium` - Annual income used for calculation
///
/// # Returns
/// The pension insurance premium in yen
fn get_pension_insurance(assessed_premium: i64) -> i64 {
    let capped_premium = assessed_premium.min(PENSION_INSURANCE_CAP);
    (capped_premium * PENSION_INSURANCE_RATE) / 10_000
}

fn calculate_take_home(
    annual_income: i64,
    num_dependents: i64,
    costs: Option<&MonthlyCosts>,
    log_config: Option<LogConfig>,
) -> (i64, Option<i64>) {
    let basic_deduction = get_basic_deduction(annual_income);
    let income_after_earned_income_deduction = annual_income - basic_deduction;

    let (national_exemption, local_exemption) = get_personal_exemptions(annual_income);
    let national_tax_basis = income_after_earned_income_deduction - national_exemption;

    let gross_national_tax_liability = get_income_tax(national_tax_basis);
    let national_surtax: i64 = gross_national_tax_liability * 210 / 10_000;
    let national_tax =
        gross_national_tax_liability + national_surtax - NATIONAL_FIXED_AMOUNT_TAX_REDUCTION;

    let local_tax_basis = income_after_earned_income_deduction - local_exemption;
    let with_equaliser_gross_tax = local_tax_basis;
    let prefectural_tax = get_prefectural_tax(with_equaliser_gross_tax);
    let municipal_tax = get_municipal_tax(with_equaliser_gross_tax);
    let local_tax = prefectural_tax + municipal_tax
        - LOCAL_FIXED_AMOUNT_TAX_REDUCTION
        - EQUALISATION_PER_CAPITA_TAX;

    let health_insurance = get_health_insurance(local_tax_basis, num_dependents);
    let pension_insurance = get_pension_insurance(local_tax_basis);
    let unemployment_insurance = get_unemployment_insurance(local_tax_basis);

    let total_tax = national_tax + local_tax;
    let percentage_of_tax_wrt_income = (total_tax as f64 / annual_income as f64) * 100.0;
    let total_insurance = health_insurance + pension_insurance + unemployment_insurance;
    let percentage_of_insurance_wrt_income =
        (total_insurance as f64 / annual_income as f64) * 100.0;
    let total_tax_and_insurance = total_tax + total_insurance;
    let percentage_of_tax_and_insurance_wrt_income =
        (total_tax_and_insurance as f64 / annual_income as f64) * 100.0;
    let net_pay = annual_income - total_tax - total_insurance;
    let percentage_of_net_pay_wrt_income = (net_pay as f64 / annual_income as f64) * 100.0;

    let monthly_take_home = net_pay / 12;

    let monthly_after_costs =
        costs.map(|c| monthly_take_home - c.calculate_total(monthly_take_home));

    if let Some(config) = log_config {
        if config.show_deductions {
            println!("Basic Deduction: {}", format_yen(basic_deduction));
        }

        if config.show_tax_breakdown {
            println!(
                "Income After Earned Income Deduction: {}",
                format_yen(income_after_earned_income_deduction)
            );
            println!("National Exemption: {}", format_yen(national_exemption));
            println!("National Tax Basis: {}", format_yen(national_tax_basis));
            println!(
                "Gross National Tax Liability: {}",
                format_yen(gross_national_tax_liability)
            );
            println!("National Surtax: {}", format_yen(national_surtax));
            println!("------");
            println!("National Tax Due: {}", format_yen(national_tax));

            println!("");
            println!("* * * * *");
            println!("");

            println!("Local Exemption: {}", format_yen(local_exemption));
            println!("Local Tax Basis: {}", format_yen(local_tax_basis));
            println!(
                "With Equalizer Gross Tax: {}",
                format_yen(with_equaliser_gross_tax)
            );
            println!("Prefectural Tax: {}", format_yen(prefectural_tax));
            println!("Municipal Tax: {}", format_yen(municipal_tax));
            println!("Local Tax: {}", format_yen(local_tax));
            println!("------");
            println!("Total Tax: {}", format_yen(total_tax));
        }

        if config.show_insurance_breakdown {
            println!("Health Insurance: {}", format_yen(health_insurance));
            println!("Pension Insurance: {}", format_yen(pension_insurance));
            println!(
                "Unemployment Insurance: {}",
                format_yen(unemployment_insurance)
            );
        }

        if config.show_summary {
            println!("Total Insurance: {}", format_yen(total_insurance));
            println!(
                "Total Tax and Insurance: {}",
                format_yen(total_tax_and_insurance)
            );
            println!(
                "Percentage of Tax and Insurance with respect to Income: {:.2}% ({:.2}% tax, {:.2}% insurance)",
                percentage_of_tax_and_insurance_wrt_income,
                percentage_of_tax_wrt_income,
                percentage_of_insurance_wrt_income
            );
            println!("Net Pay: {}", format_yen(net_pay));
            println!(
                "Percentage of Net Pay: {:.2}%",
                percentage_of_net_pay_wrt_income
            );

            println!("Monthly take home: {}", format_yen(monthly_take_home));
        }
    }

    (monthly_take_home, monthly_after_costs)
}

fn analyze_income(
    income: IncomeAnalysis,
    comparative_income: Option<i64>,
    num_dependents: i64,
    timeframes: &[SavingsTimeframe],
    is_first: bool,
) -> i64 {
    let monthly_salary = income.annual_income / 12;
    let (monthly_take_home, monthly_after_costs) = calculate_take_home(
        income.annual_income,
        num_dependents,
        income.monthly_costs.as_ref(),
        income.log_config,
    );

    let total_costs = monthly_after_costs.map(|after_costs| monthly_take_home - after_costs);

    // Print header if this is the first item
    if is_first {
        let mut header = format!(
            "{:<12} | {:<16} | {:<16} | {:<21} | {:<23} | {:<15}",
            "Annual Salary",
            "Monthly Salary",
            "Monthly Takehome",
            "Takehome Increase (%)",
            "Total Costs (Variable)",
            "After Costs"
        );

        // Add timeframe columns to the same header line
        for timeframe in timeframes {
            header.push_str(&format!(
                " | {:<18}",
                format!("Saved in {}", timeframe.label)
            ));
        }
        println!("{}", header);

        // Calculate total width and print single separator line
        println!("{:-<width$}", "", width = header.len());
    }

    let formatted_annual = format!("¥{}M", income.annual_income / 1_000_000);

    if let Some(comparative) = comparative_income {
        let (comparative_monthly, _) = calculate_take_home(
            comparative,
            num_dependents,
            income.monthly_costs.as_ref(),
            None,
        );
        let increase = monthly_take_home - comparative_monthly;
        let percentage = ((monthly_take_home as f64 - comparative_monthly as f64)
            / comparative_monthly as f64)
            * 100.0;

        let costs_display = total_costs.map_or("N/A".to_string(), |c| {
            if let Some(costs) = &income.monthly_costs {
                let variable_costs =
                    (monthly_take_home as f64 * costs.percentage_costs / 100.0) as i64;
                format!("{} ({})", format_yen(c), format_yen(variable_costs))
            } else {
                format_yen(c)
            }
        });

        // Print base columns
        print!(
            "{:<13} | {:<16} | {:<16} | {:<21} | {:<23} | {:<15}",
            formatted_annual,
            format_yen(monthly_salary),
            format_yen(monthly_take_home),
            format!("{} ({:.2}%)", format_yen(increase), percentage),
            costs_display,
            monthly_after_costs.map_or("N/A".to_string(), |c| format_yen(c))
        );

        // Add savings columns
        if let Some(after_costs) = monthly_after_costs {
            for timeframe in timeframes {
                let savings = after_costs * timeframe.months;
                print!(" | {:<18}", format_yen(savings));
            }
        } else {
            for _ in timeframes {
                print!(" | {:<18}", "N/A");
            }
        }
        println!();
    } else {
        println!(
            "{:<15} | {:<18} | {:<18} | {:<23} | {:<18}",
            formatted_annual,
            format_yen(monthly_salary),
            format_yen(monthly_take_home),
            format!("{} ({:.2}%)", format_yen(0), 0.00),
            monthly_after_costs.map_or("N/A".to_string(), |c| format_yen(c))
        );
    }

    monthly_take_home
}

fn main() {
    let num_dependents = 2; // 2 dependents
    let comparative_income = Some(15_000_000); // JPY

    let timeframes = vec![
        SavingsTimeframe {
            months: 3,
            label: "3 Months".to_string(),
        },
        SavingsTimeframe {
            months: 6,
            label: "6 Months".to_string(),
        },
        SavingsTimeframe {
            months: 12,
            label: "1 Year".to_string(),
        },
        SavingsTimeframe {
            months: 24,
            label: "2 Years".to_string(),
        },
        SavingsTimeframe {
            months: 60,
            label: "5 Years".to_string(),
        },
    ];

    let income_levels = vec![
        IncomeAnalysis {
            annual_income: 15_000_000,
            monthly_costs: Some(MonthlyCosts {
                fixed_costs: 750_000,
                percentage_costs: 0.0,
            }),
            log_config: Some(LogConfig::none()),
        },
        IncomeAnalysis {
            annual_income: 18_000_000,
            monthly_costs: Some(MonthlyCosts {
                fixed_costs: 750_000,
                percentage_costs: 10.0,
            }),
            log_config: Some(LogConfig::none()),
        },
        IncomeAnalysis {
            annual_income: 20_000_000,
            monthly_costs: Some(MonthlyCosts {
                fixed_costs: 750_000,
                percentage_costs: 10.0,
            }),
            log_config: Some(LogConfig::none()),
        },
        IncomeAnalysis {
            annual_income: 22_000_000,
            monthly_costs: Some(MonthlyCosts {
                fixed_costs: 750_000,
                percentage_costs: 10.0,
            }),
            log_config: Some(LogConfig::none()),
        },
        IncomeAnalysis {
            annual_income: 25_000_000,
            monthly_costs: Some(MonthlyCosts {
                fixed_costs: 750_000,
                percentage_costs: 10.0,
            }),
            log_config: Some(LogConfig::none()),
        },
        IncomeAnalysis {
            annual_income: 30_000_000,
            monthly_costs: Some(MonthlyCosts {
                fixed_costs: 750_000,
                percentage_costs: 10.0,
            }),
            log_config: Some(LogConfig::none()),
        },
        IncomeAnalysis {
            annual_income: 100_000_000,
            monthly_costs: Some(MonthlyCosts {
                fixed_costs: 750_000,
                percentage_costs: 10.0,
            }),
            log_config: Some(LogConfig::none()),
        },
    ];

    for (index, income) in income_levels.into_iter().enumerate() {
        analyze_income(
            income,
            comparative_income,
            num_dependents,
            &timeframes,
            index == 0,
        );
    }
}
