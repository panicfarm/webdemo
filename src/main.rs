use {
    chaindata::db::Db,
    hyper::{
        server::conn::AddrStream,
        // Following functions are used by Hyper to handle a `Request`
        // and returning a `Response` in an asynchronous manner by using a Future
        service::{make_service_fn, service_fn},
        // Miscellaneous types from Hyper for working with HTTP.
        Body,
        Request,
        Response,
        Server,
    },
    serde::{Deserialize, Serialize},
    std::fmt::Debug,
    std::net::SocketAddr,
    std::path::PathBuf,
    tokio::task,
};

#[derive(Serialize, Deserialize, Debug)]
struct GraphReq {
    vx_vec: Vec<u32>,
    bl_min: u32,
    bl_max: u32,
    flux_threshold: u64,
}

async fn serve_req(req: Request<Body>) -> Result<Response<Body>, hyper::Error> {
    println!("Request {:?}", req);
    match req.uri().path() {
        "/graph" => {
            let bytes = hyper::body::to_bytes(req.into_body()).await?;
            let graph_req: GraphReq = serde_json::from_slice(&bytes).unwrap();
            println!("{:?}", graph_req);
            let res = task::spawn_blocking(move || {
                let db_path = "/home/alecm/clustering/lmdb.100000";
                let dp = PathBuf::from(db_path);
                let db = Db::new(&dp, 100_000_000_000, false).unwrap();
                let graph = db.create_graph_adapter().unwrap();
                let edges = graph
                    .edges(
                        graph_req.vx_vec,
                        graph_req.bl_min,
                        graph_req.bl_max,
                        graph_req.flux_threshold,
                        2,
                    )
                    .unwrap();
                serde_json::to_string(&edges).unwrap()
            })
            .await
            .unwrap();
            Ok(Response::new(Body::from(res)))
        }
        _ => Ok(Response::new(Body::from(String::from_utf8_lossy(
            include_bytes!("../files/index.html"),
        )))),
    }
}

async fn run_server(addr: SocketAddr, x: u8) {
    println!("Listening on http://{}", addr);

    // Create a server bound on the provided address
    let serve_future = Server::bind(&addr)
        // Serve requests using our `async serve_req` function.
        // `serve` takes a type which implements the `MakeService` trait.
        // `make_service_fn` converts a closure into a type which
        // implements the `MakeService` trait. That closure must return a
        // type that implements the `Service` trait, and `service_fn`
        // converts a request-response function into a type that implements
        // the `Service` trait.
        .serve(make_service_fn(|socket: &AddrStream| {
            println!("{} {}", socket.remote_addr(), x);
            async { Ok::<_, hyper::Error>(service_fn(serve_req)) }
        }));

    // Wait for the server to complete serving or exit with an error.
    // If an error occurred, print it to stderr.
    if let Err(e) = serve_future.await {
        eprintln!("server error: {}", e);
    }
}

#[tokio::main]
async fn main() {
    // Set the address to run our socket on.
    let addr = SocketAddr::from(([208, 93, 231, 240], 3000));
    let x = 3;

    // Call our `run_server` function, which returns a future.
    // As with every `async fn`, for `run_server` to do anything,
    // the returned future needs to be run using `await`;
    run_server(addr, x).await;
}
