pub mod identity;
pub mod rooms;
pub mod messages;
pub mod members;
pub mod files;
pub mod peers;

use actix_web::web;

/// Register all REST API routes under `/v1/`.
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg
        // Identity
        .route("/v1/identity", web::get().to(identity::get_identity))
        .route("/v1/identity/sign", web::post().to(identity::sign_payload))

        // Rooms
        .route("/v1/rooms", web::get().to(rooms::list_rooms))
        .route("/v1/rooms", web::post().to(rooms::create_room))
        .route("/v1/rooms/{room_id}", web::get().to(rooms::get_room))
        .route("/v1/rooms/{room_id}", web::patch().to(rooms::patch_room))
        .route("/v1/rooms/{room_id}", web::delete().to(rooms::delete_room))

        // Messages
        .route("/v1/rooms/{room_id}/messages", web::get().to(messages::list_messages))
        .route("/v1/rooms/{room_id}/messages", web::post().to(messages::send_message))
        .route(
            "/v1/rooms/{room_id}/messages/{event_id}/react",
            web::post().to(messages::react),
        )
        .route(
            "/v1/rooms/{room_id}/messages/{event_id}",
            web::patch().to(messages::edit_message),
        )
        .route(
            "/v1/rooms/{room_id}/messages/{event_id}",
            web::delete().to(messages::delete_message),
        )

        // Members
        .route("/v1/rooms/{room_id}/members", web::post().to(members::invite_member))
        .route(
            "/v1/rooms/{room_id}/members/{address}",
            web::patch().to(members::change_role),
        )
        .route(
            "/v1/rooms/{room_id}/members/{address}",
            web::delete().to(members::kick_member),
        )

        // Files
        .route("/v1/rooms/{room_id}/files", web::post().to(files::upload_file))
        .route(
            "/v1/rooms/{room_id}/files/{file_id}",
            web::get().to(files::download_file),
        )

        // Peers
        .route("/v1/peers", web::get().to(peers::list_peers))
        .route("/v1/peers/rendezvous", web::get().to(peers::list_rendezvous));
}
