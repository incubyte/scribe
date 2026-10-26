const PLACEHOLDER_UNTIL_ENTRA_APP_REGISTRATION: &str = "00000000-0000-0000-0000-000000000000";

pub const CLIENT_ID: &str = PLACEHOLDER_UNTIL_ENTRA_APP_REGISTRATION;
pub const AUTHORIZE_URL: &str =
    "https://login.microsoftonline.com/organizations/oauth2/v2.0/authorize";
pub const TOKEN_URL: &str = "https://login.microsoftonline.com/organizations/oauth2/v2.0/token";
pub const SCOPES: &str = "openid profile offline_access Calendars.ReadBasic";
pub const GRAPH_ORIGIN: &str = "https://graph.microsoft.com/";
pub const CALENDAR_VIEW_URL: &str = "https://graph.microsoft.com/v1.0/me/calendarView";
