use super::{FunctionHandler, FunctionRegistration};
use super::super::catalog::{ExpressionFunctionDescriptor, P_ANY, P_STRING2, R_BOOL, R_STRING_ERR};
use crate::catalog::{Example, ExampleVerification};

pub(super) const REGISTRATIONS: &[FunctionRegistration] = &[
    FunctionRegistration { canonical: "date", aliases: &[], catalog_order: 31, descriptors: &[
        ExpressionFunctionDescriptor {

                signature: "date(iso, fmt)",
                parameters: P_STRING2,
                returns: R_STRING_ERR,
                description: "Reformats an ISO date/datetime string into a named human format.",
                category: "Date Formatting",
                order: 1,

                example: Some(Example { invocation: "date(\"2024-06-15\", \"long\")", result: "Sat, June 15th, 2024", verification: ExampleVerification::Executable }),

            },
    ], handler: FunctionHandler::Pure(super::date_fn) },
    FunctionRegistration { canonical: "is_date", aliases: &["isdate"], catalog_order: 32, descriptors: &[
        ExpressionFunctionDescriptor {

                signature: "is_date(x)",
                parameters: P_ANY,
                returns: R_BOOL,
                description: "Returns true when the string is a valid ISO date (YYYY-MM-DD).",
                category: "Date Validators",
                order: 1,

                example: Some(Example { invocation: "is_date(\"2024-06-15\")", result: "true", verification: ExampleVerification::Executable }),

            },
    ], handler: FunctionHandler::Pure(super::is_date) },
    FunctionRegistration { canonical: "is_date_utc", aliases: &["isdateutc"], catalog_order: 33, descriptors: &[
        ExpressionFunctionDescriptor {

                signature: "is_date_utc(x)",
                parameters: P_ANY,
                returns: R_BOOL,
                description: "Same as is_date (the format itself is timezone-agnostic).",
                category: "Date Validators",
                order: 2,

                example: Some(Example { invocation: "is_date_utc(\"2024-06-15\")", result: "true", verification: ExampleVerification::Executable }),

            },
    ], handler: FunctionHandler::Pure(super::is_date_utc) },
    FunctionRegistration { canonical: "is_date_time", aliases: &["isdatetime", "is_datetime"], catalog_order: 34, descriptors: &[
        ExpressionFunctionDescriptor {

                signature: "is_date_time(x)",
                parameters: P_ANY,
                returns: R_BOOL,
                description: "Returns true when the string is a valid ISO datetime.",
                category: "Date Validators",
                order: 3,

                example: Some(Example { invocation: "is_date_time(\"2024-06-15T12:30:00\")", result: "true", verification: ExampleVerification::Executable }),

            },
    ], handler: FunctionHandler::Pure(super::is_datetime) },
    FunctionRegistration { canonical: "is_date_time_utc", aliases: &["isdatetimeutc", "is_datetime_utc"], catalog_order: 35, descriptors: &[
        ExpressionFunctionDescriptor {

                signature: "is_date_time_utc(x)",
                parameters: P_ANY,
                returns: R_BOOL,
                description: "Same parse contract as is_date_time.",
                category: "Date Validators",
                order: 4,

                example: Some(Example { invocation: "is_date_time_utc(\"2024-06-15T12:30:00Z\")", result: "true", verification: ExampleVerification::Executable }),

            },
    ], handler: FunctionHandler::Pure(super::is_datetime_utc) },
    FunctionRegistration { canonical: "is_today", aliases: &["istoday"], catalog_order: 36, descriptors: &[
        ExpressionFunctionDescriptor {

                signature: "is_today(x)",
                parameters: P_ANY,
                returns: R_BOOL,
                description: "Returns true when the date/datetime is today (local).",
                category: "Date Validators",
                order: 5,

                example: Some(Example { invocation: "is_today(\"2024-06-15\")", result: "true", verification: ExampleVerification::DisplayOnly("wall-clock dependent") }),

            },
    ], handler: FunctionHandler::Pure(super::is_today) },
    FunctionRegistration { canonical: "is_today_utc", aliases: &["istodayutc"], catalog_order: 37, descriptors: &[
        ExpressionFunctionDescriptor {

                signature: "is_today_utc(x)",
                parameters: P_ANY,
                returns: R_BOOL,
                description: "Returns true when the date/datetime is today (UTC).",
                category: "Date Validators",
                order: 6,

                example: Some(Example { invocation: "is_today_utc(\"2024-06-15\")", result: "true", verification: ExampleVerification::DisplayOnly("wall-clock dependent") }),

            },
    ], handler: FunctionHandler::Pure(super::is_today_utc) },
    FunctionRegistration { canonical: "is_yesterday", aliases: &["isyesterday"], catalog_order: 38, descriptors: &[
        ExpressionFunctionDescriptor {

                signature: "is_yesterday(x)",
                parameters: P_ANY,
                returns: R_BOOL,
                description: "Returns true when the date/datetime is yesterday (local).",
                category: "Date Validators",
                order: 7,

                example: Some(Example { invocation: "is_yesterday(\"2024-06-14\")", result: "true", verification: ExampleVerification::DisplayOnly("wall-clock dependent") }),

            },
    ], handler: FunctionHandler::Pure(super::is_yesterday) },
    FunctionRegistration { canonical: "is_yesterday_utc", aliases: &["isyesterdayutc"], catalog_order: 39, descriptors: &[
        ExpressionFunctionDescriptor {

                signature: "is_yesterday_utc(x)",
                parameters: P_ANY,
                returns: R_BOOL,
                description: "Returns true when the date/datetime is yesterday (UTC).",
                category: "Date Validators",
                order: 8,

                example: Some(Example { invocation: "is_yesterday_utc(\"2024-06-14\")", result: "true", verification: ExampleVerification::DisplayOnly("wall-clock dependent") }),

            },
    ], handler: FunctionHandler::Pure(super::is_yesterday_utc) },
    FunctionRegistration { canonical: "is_tomorrow", aliases: &["istomorrow"], catalog_order: 40, descriptors: &[
        ExpressionFunctionDescriptor {

                signature: "is_tomorrow(x)",
                parameters: P_ANY,
                returns: R_BOOL,
                description: "Returns true when the date/datetime is tomorrow (local).",
                category: "Date Validators",
                order: 9,

                example: Some(Example { invocation: "is_tomorrow(\"2024-06-16\")", result: "true", verification: ExampleVerification::DisplayOnly("wall-clock dependent") }),

            },
    ], handler: FunctionHandler::Pure(super::is_tomorrow) },
    FunctionRegistration { canonical: "is_tomorrow_utc", aliases: &["istomorrowutc"], catalog_order: 41, descriptors: &[
        ExpressionFunctionDescriptor {

                signature: "is_tomorrow_utc(x)",
                parameters: P_ANY,
                returns: R_BOOL,
                description: "Returns true when the date/datetime is tomorrow (UTC).",
                category: "Date Validators",
                order: 10,

                example: Some(Example { invocation: "is_tomorrow_utc(\"2024-06-16\")", result: "true", verification: ExampleVerification::DisplayOnly("wall-clock dependent") }),

            },
    ], handler: FunctionHandler::Pure(super::is_tomorrow_utc) },
    FunctionRegistration { canonical: "is_this_month", aliases: &["isthismonth"], catalog_order: 42, descriptors: &[
        ExpressionFunctionDescriptor {

                signature: "is_this_month(x)",
                parameters: P_ANY,
                returns: R_BOOL,
                description: "Returns true when the date/datetime is in the current month (local).",
                category: "Date Validators",
                order: 11,

                example: Some(Example { invocation: "is_this_month(\"2024-06-15\")", result: "true", verification: ExampleVerification::DisplayOnly("wall-clock dependent") }),

            },
    ], handler: FunctionHandler::Pure(super::is_this_month) },
    FunctionRegistration { canonical: "is_this_month_utc", aliases: &["isthismonthutc"], catalog_order: 43, descriptors: &[
        ExpressionFunctionDescriptor {

                signature: "is_this_month_utc(x)",
                parameters: P_ANY,
                returns: R_BOOL,
                description: "Returns true when the date/datetime is in the current month (UTC).",
                category: "Date Validators",
                order: 12,

                example: Some(Example { invocation: "is_this_month_utc(\"2024-06-15\")", result: "true", verification: ExampleVerification::DisplayOnly("wall-clock dependent") }),

            },
    ], handler: FunctionHandler::Pure(super::is_this_month_utc) },
    FunctionRegistration { canonical: "is_this_year", aliases: &["isthisyear"], catalog_order: 44, descriptors: &[
        ExpressionFunctionDescriptor {

                signature: "is_this_year(x)",
                parameters: P_ANY,
                returns: R_BOOL,
                description: "Returns true when the date/datetime is in the current year (local).",
                category: "Date Validators",
                order: 13,

                example: Some(Example { invocation: "is_this_year(\"2024-06-15\")", result: "true", verification: ExampleVerification::DisplayOnly("wall-clock dependent") }),

            },
    ], handler: FunctionHandler::Pure(super::is_this_year) },
    FunctionRegistration { canonical: "is_this_year_utc", aliases: &["isthisyearutc"], catalog_order: 45, descriptors: &[
        ExpressionFunctionDescriptor {

                signature: "is_this_year_utc(x)",
                parameters: P_ANY,
                returns: R_BOOL,
                description: "Returns true when the date/datetime is in the current year (UTC).",
                category: "Date Validators",
                order: 14,

                example: Some(Example { invocation: "is_this_year_utc(\"2024-06-15\")", result: "true", verification: ExampleVerification::DisplayOnly("wall-clock dependent") }),

            },
    ], handler: FunctionHandler::Pure(super::is_this_year_utc) },
];
