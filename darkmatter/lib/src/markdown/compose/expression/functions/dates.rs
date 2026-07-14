use super::{EvaluationMode, FunctionBinding, FunctionHandler};

pub(super) const BINDINGS: &[FunctionBinding] = &[
    FunctionBinding { canonical: "date", aliases: &[], evaluation: EvaluationMode::Pure, handler: Some(FunctionHandler::Pure(super::date_fn)) },
    FunctionBinding { canonical: "is_date", aliases: &["isdate"], evaluation: EvaluationMode::Pure, handler: Some(FunctionHandler::Pure(super::is_date)) },
    FunctionBinding { canonical: "is_date_utc", aliases: &["isdateutc"], evaluation: EvaluationMode::Pure, handler: Some(FunctionHandler::Pure(super::is_date_utc)) },
    FunctionBinding { canonical: "is_date_time", aliases: &["isdatetime", "is_datetime"], evaluation: EvaluationMode::Pure, handler: Some(FunctionHandler::Pure(super::is_datetime)) },
    FunctionBinding { canonical: "is_date_time_utc", aliases: &["isdatetimeutc", "is_datetime_utc"], evaluation: EvaluationMode::Pure, handler: Some(FunctionHandler::Pure(super::is_datetime_utc)) },
    FunctionBinding { canonical: "is_today", aliases: &["istoday"], evaluation: EvaluationMode::Pure, handler: Some(FunctionHandler::Pure(super::is_today)) },
    FunctionBinding { canonical: "is_today_utc", aliases: &["istodayutc"], evaluation: EvaluationMode::Pure, handler: Some(FunctionHandler::Pure(super::is_today_utc)) },
    FunctionBinding { canonical: "is_yesterday", aliases: &["isyesterday"], evaluation: EvaluationMode::Pure, handler: Some(FunctionHandler::Pure(super::is_yesterday)) },
    FunctionBinding { canonical: "is_yesterday_utc", aliases: &["isyesterdayutc"], evaluation: EvaluationMode::Pure, handler: Some(FunctionHandler::Pure(super::is_yesterday_utc)) },
    FunctionBinding { canonical: "is_tomorrow", aliases: &["istomorrow"], evaluation: EvaluationMode::Pure, handler: Some(FunctionHandler::Pure(super::is_tomorrow)) },
    FunctionBinding { canonical: "is_tomorrow_utc", aliases: &["istomorrowutc"], evaluation: EvaluationMode::Pure, handler: Some(FunctionHandler::Pure(super::is_tomorrow_utc)) },
    FunctionBinding { canonical: "is_this_month", aliases: &["isthismonth"], evaluation: EvaluationMode::Pure, handler: Some(FunctionHandler::Pure(super::is_this_month)) },
    FunctionBinding { canonical: "is_this_month_utc", aliases: &["isthismonthutc"], evaluation: EvaluationMode::Pure, handler: Some(FunctionHandler::Pure(super::is_this_month_utc)) },
    FunctionBinding { canonical: "is_this_year", aliases: &["isthisyear"], evaluation: EvaluationMode::Pure, handler: Some(FunctionHandler::Pure(super::is_this_year)) },
    FunctionBinding { canonical: "is_this_year_utc", aliases: &["isthisyearutc"], evaluation: EvaluationMode::Pure, handler: Some(FunctionHandler::Pure(super::is_this_year_utc)) },
    FunctionBinding { canonical: "date_delta", aliases: &["datedelta"], evaluation: EvaluationMode::Pure, handler: Some(FunctionHandler::Pure(super::date_delta)) },
    FunctionBinding { canonical: "older_than", aliases: &["olderthan"], evaluation: EvaluationMode::Pure, handler: Some(FunctionHandler::Pure(super::older_than)) },
    FunctionBinding { canonical: "newer_than", aliases: &["newerthan"], evaluation: EvaluationMode::Pure, handler: Some(FunctionHandler::Pure(super::newer_than)) },
];
