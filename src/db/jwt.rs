use base64_url::encode;
use chrono::Utc;
use openssl::{hash::MessageDigest, pkey::PKey, sign::Signer};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::error::Error;
use std::{fs::File, io::BufReader, path::Path};

pub const ASSERTION_TARGET: &str = "https://firestore.googleapis.com/";
const SCOPE: &str = "https://www.googleapis.com/auth/datastore";
const ONE_HOUR: i64 = 3600;

/// Creates a JWT.
///
/// Requires a GCP service account credential file in json format.
///
/// TODO: this could be documented better
///
/// Combines a claim set with a JWT header to calculates a signature.
/// The JWT claim set contains information about the JWT,
/// including the permissions being requested (scopes), the target of the token,
/// the issuer, the time the token was issued, and the lifetime of the token.
pub fn generate(c: &CredentialJsonFile) -> String {
    //let email_field = "rusty-bot@ut4-hubs.iam.gserviceaccount.com".to_string();
    let pk_field = "-----BEGIN PRIVATE KEY-----\nMIIEvQIBADANBgkqhkiG9w0BAQEFAASCBKcwggSjAgEAAoIBAQCEw6kxiuwXviht\nB7eUo824M8q7W/SEhZ8BqohlEytWWp9CQaDyTUhT6nzwQGd+ET4ElnwGKytg5sWo\nLDwu/Z80KMuGIEiwqeQI42Fx50R/MJv725iRTzj/2tkKbkCYWDYiqaWgY5yWts4Q\ndOvzK6WbawgxSgm9L6rmCSRRNsyLlYaQGs3GiQlaTDi6bIzfrH7oyHXxMYbTSm3+\nckqtvpRtLlP31SD461eiy8hKpxUQ1zrvqVSO6LS7YNWEvDh0xN/g/32HHlu7V4OL\nT/X1i5hj2ZVuMX7bpiU8NOmsgBeERgPKL34UqVx9VRj+RV0i/yOOGBo/k6VbHYHd\nBfTgOJfFAgMBAAECggEACBPtkfWdas5noEalbVZQGIKSNbchGAUXSR8qoFe7vcvS\nv9HFbKi2G2D7jBSnF10ONHJBhpCO2Z0A7rwOR1+oFaDbzUPemF0l4kKPdcI4ATMg\ncoEIdeLnmW2j4gYWSQ6o8I/441rcNrFVtVuf1ZJUx7GJ9JHIKOojEE8DFivq4x01\n/Pq501mOoI7QOnOB5OMr8CXE5hCfXIICJX8TEDRCLGXZn4YJnnBQvpJRtEibtEBT\nR6mUgSGkvDtVNyHhJx5Tfk09Y2MHjW6TZojvtt40Fkr9t+WOA/5zYCi16ehtV0RK\nibc5jHaFPe/g9BwT7zIP0fnMjpf2MvtOPxu4gx7/EQKBgQC5jABJDJeHmLkWYNaY\nUVzbc7pcSjUJNUluxRoGPSyFBzSuwGsKrgDD4dVRUaS4+OhTK1XY5Uau7gJWIJM5\nJLQrp0cdbJMl/+x4gYSOq3JP1kXWxWzykhjKoH0dNwTOovuLZBqqFu4v7kzkU/di\n5nImMg5NhHtZTiCBI/NGSZUFiQKBgQC3LPQbXkW9U7L9zYRXl5V3x8OoF4+MBZmX\niijBSsSP0lgluN1Zo/+4MeM4DKqpl7kBxNlENjkH9gD18KAByNwuQz0xviHcTZhA\nSRtZb0a1TyMSaoox/GrShysQ9BA208SoGXDoK5DigYrL/KPPlvpCFXlg2TxkIS3Y\nSDXbL5itXQKBgFYIqIk2oXxqQEg0Fs0BzQCkpKDud7ERWD9YfTyvWNlGAhOVfQyy\nqgAp0vOl2685GuCVk3TCuweZrNOqvxkb/77ODZeDJKfWBxvJUmGk9Zg3TqLLYD0J\nqR0rVVgajswRnnl/rS14/HCVGmo01Nyy5fL3+tHOwDMkmsXGmaLZs5OhAoGAWFdm\n1zgtHDUcswkGFZR2spD2TMAoK9ibjZlFNIuRpudEOdUhc9UDRFtTAToiqK4SvMaq\nHhqkgLFlHzfQg2vSvMES50WSYQRPNFnNxeFD0bd766rUQW1CO4yS+ZcrprWVN5kl\neeNg+cyOGvkaK8jdozdmFY5PcN8LlC6nQrF+ycUCgYEAiiuHaNU23YFyqQp6ZY6v\nW1hiat6y0c9sJEx00DZzEYb2HKIlDvnq/P/vhW9OQW+CBDKbAieq2/HpYftUnsIA\n+Lad8oS6oKP4bR2Sl51ynorLUEYfURGT010z3NXtvFhypfmZLhXHoQ282rjstAic\n2CcUo9bAG1v41gWay22647I=\n-----END PRIVATE KEY-----\n".to_string();
    let pem_pk = c.private_key.as_bytes(); // pk_field.as_bytes();

    let now = Utc::now().timestamp();
    let jwt_header = json!({
        "alg":"RS256",
        "typ":"JWT",
        "kid": c.private_key_id
    })
    .to_string();
    let claim_set = json!({
        // The email address of the service account
        "iss": c.client_email,
        "sub": c.client_email,
        // A descriptor of the intended target of the assertion.
        // When making an access token request this value is always `https://oauth2.googleapis.com/token`
        "aud": "https://firestore.googleapis.com/",
        // The expiration time of the assertion, specified as seconds since 00:00:00 UTC, January 1, 1970
        // This value has a maximum of 1 hour after the issued time
        "exp": now + ONE_HOUR,
        // The time the assertion was issued, specified as seconds since 00:00:00 UTC, January 1, 1970
        "iat": now,
    })
    .to_string();

    let input = format!("{}.{}", encode(&jwt_header), encode(&claim_set));
    let private_key = PKey::private_key_from_pem(pem_pk).unwrap();
    let mut s = Signer::new(MessageDigest::sha256(), &private_key).unwrap();
    let signature = s
        .sign_oneshot_to_vec(input.as_bytes())
        .expect("Expected signing to succeed");
    format!("{}.{}", input, encode(&signature))
}

pub fn read_credentials_from_file<P: AsRef<Path>>(
    path: P,
) -> Result<CredentialJsonFile, Box<dyn Error>> {
    // Open the file in read-only mode with buffer.
    let file = File::open(path)?;
    let reader = BufReader::new(file);

    // Read the JSON contents of the file as an instance of `CredentialJsonFile`.
    let c = serde_json::from_reader(reader)?;

    // Return the `CredentialJsonFile`.
    Ok(c)
}

#[derive(Serialize, Deserialize)]
pub struct CredentialJsonFile {
    project_id: String,
    private_key_id: String,
    private_key: String,
    client_email: String,
    auth_uri: String,
    token_uri: String,
}
