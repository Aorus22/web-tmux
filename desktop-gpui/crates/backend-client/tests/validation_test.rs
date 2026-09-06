use webtmux_backend_client::{validate_session_name, ValidationError};

#[test]
fn test_session_name_validation() {
    // Valid session names per Go ValidateSessionName regex ^[A-Za-z0-9][A-Za-z0-9_./-]*$
    assert!(validate_session_name("dev").is_ok());
    assert!(validate_session_name("my_session-1").is_ok());
    assert!(validate_session_name("session/with/slashes").is_ok());
    assert!(validate_session_name("Session_123").is_ok());
    assert!(validate_session_name("0").is_ok());

    // Empty name
    assert_eq!(validate_session_name(""), Err(ValidationError::Empty));

    // Too long (>200 chars)
    let long_name = "a".repeat(201);
    assert_eq!(validate_session_name(&long_name), Err(ValidationError::TooLong));

    // Contains ':' or '.'
    assert_eq!(
        validate_session_name("alpha:bravo"),
        Err(ValidationError::ContainsColonOrDot)
    );
    assert_eq!(
        validate_session_name("alpha.bravo"),
        Err(ValidationError::ContainsColonOrDot)
    );
    assert_eq!(
        validate_session_name("sess:1"),
        Err(ValidationError::ContainsColonOrDot)
    );

    // Starts with '$'
    assert_eq!(
        validate_session_name("$session"),
        Err(ValidationError::StartsWithDollar)
    );

    // Invalid characters (spaces, special chars)
    assert_eq!(
        validate_session_name("my session"),
        Err(ValidationError::InvalidCharacters("my session".to_string()))
    );
    assert_eq!(
        validate_session_name("-starts-with-hyphen"),
        Err(ValidationError::InvalidCharacters("-starts-with-hyphen".to_string()))
    );
    assert_eq!(
        validate_session_name("_starts_with_underscore"),
        Err(ValidationError::InvalidCharacters("_starts_with_underscore".to_string()))
    );
    assert_eq!(
        validate_session_name("session@name"),
        Err(ValidationError::InvalidCharacters("session@name".to_string()))
    );
}
