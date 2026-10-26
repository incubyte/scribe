const PLACEHOLDER_UNTIL_ENTRA_APP_REGISTRATION: &str = "00000000-0000-0000-0000-000000000000";

pub const CLIENT_ID: &str = PLACEHOLDER_UNTIL_ENTRA_APP_REGISTRATION;
pub const AUTHORIZE_URL: &str =
    "https://login.microsoftonline.com/organizations/oauth2/v2.0/authorize";
pub const TOKEN_URL: &str = "https://login.microsoftonline.com/organizations/oauth2/v2.0/token";
pub const SCOPES: &str = "openid profile offline_access Calendars.ReadBasic";
