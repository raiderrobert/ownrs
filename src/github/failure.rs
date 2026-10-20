use reqwest::StatusCode;

#[derive(Debug, PartialEq, Eq)]
pub enum Failure {
    NotFound,
    Forbidden,
    EmptyRepository,
    Unknown,
}

pub fn classify(error: &octocrab::Error) -> Failure {
    match error {
        octocrab::Error::GitHub { source, .. } => {
            from_response(source.status_code, &source.message)
        }
        _ => Failure::Unknown,
    }
}

pub fn describe(error: &octocrab::Error) -> String {
    match error {
        octocrab::Error::GitHub { source, .. } => {
            format!("{}: {}", source.status_code, source.message)
        }
        other => other.to_string(),
    }
}

fn from_response(status: StatusCode, message: &str) -> Failure {
    match status {
        StatusCode::NOT_FOUND => Failure::NotFound,
        StatusCode::FORBIDDEN if !message.to_ascii_lowercase().contains("rate limit") => {
            Failure::Forbidden
        }
        StatusCode::CONFLICT => Failure::EmptyRepository,
        _ => Failure::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_github_responses() {
        let cases = [
            (StatusCode::NOT_FOUND, "Not Found", Failure::NotFound),
            (
                StatusCode::FORBIDDEN,
                "Resource not accessible by integration",
                Failure::Forbidden,
            ),
            (
                StatusCode::FORBIDDEN,
                "API rate limit exceeded for user ID 1.",
                Failure::Unknown,
            ),
            (
                StatusCode::FORBIDDEN,
                "You have exceeded a secondary rate limit.",
                Failure::Unknown,
            ),
            (
                StatusCode::TOO_MANY_REQUESTS,
                "API rate limit exceeded for user ID 1.",
                Failure::Unknown,
            ),
            (
                StatusCode::CONFLICT,
                "Git Repository is empty.",
                Failure::EmptyRepository,
            ),
            (
                StatusCode::UNAUTHORIZED,
                "Bad credentials",
                Failure::Unknown,
            ),
            (StatusCode::BAD_GATEWAY, "Server Error", Failure::Unknown),
        ];

        for (status, message, want) in cases {
            assert_eq!(from_response(status, message), want, "{status} {message}");
        }
    }
}
