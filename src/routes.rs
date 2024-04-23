use actix_web::web::ServiceConfig;
use crate::services::serve;

pub fn routes(config: &mut ServiceConfig) {
    
    config
        .service(serve);
    
}