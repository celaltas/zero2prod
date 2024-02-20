use actix_web::{web, HttpResponse, Responder};

#[derive(serde::Deserialize, Debug)]
pub struct Parameters {
    subscription_token: String,
}

#[tracing::instrument(name = "Confirm a pending subscriber", skip(parameters))]
pub async fn confirm(parameters: web::Query<Parameters>) -> impl Responder {

    println!("parameters: {:#?}", parameters);


    HttpResponse::Ok().finish()
}
