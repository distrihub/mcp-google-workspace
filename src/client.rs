use google_drive3::DriveHub;
use google_sheets4::Sheets;

pub fn get_drive_client(
    access_token: &str,
) -> DriveHub<
    google_drive3::hyper_rustls::HttpsConnector<
        google_drive3::hyper_util::client::legacy::connect::HttpConnector,
    >,
> {
    let hub = DriveHub::new(
        google_drive3::hyper_util::client::legacy::Client::builder(
            google_drive3::hyper_util::rt::TokioExecutor::new(),
        )
        .build(
            google_sheets4::hyper_rustls::HttpsConnectorBuilder::new()
                .with_native_roots()
                .unwrap()
                .https_or_http()
                .enable_http1()
                .build(),
        ),
        access_token.to_string(),
    );
    hub
}

pub fn get_sheets_client(
    access_token: &str,
) -> Sheets<
    google_sheets4::hyper_rustls::HttpsConnector<
        google_sheets4::hyper_util::client::legacy::connect::HttpConnector,
    >,
> {
    let hub = Sheets::new(
        google_sheets4::hyper_util::client::legacy::Client::builder(
            google_sheets4::hyper_util::rt::TokioExecutor::new(),
        )
        .build(
            google_sheets4::hyper_rustls::HttpsConnectorBuilder::new()
                .with_native_roots()
                .unwrap()
                .https_or_http()
                .enable_http1()
                .build(),
        ),
        access_token.to_string(),
    );
    hub
}
