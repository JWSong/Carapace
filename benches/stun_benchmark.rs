use criterion::{Criterion, Throughput, black_box, criterion_group, criterion_main};
use std::net::{Ipv4Addr, SocketAddrV4};

use carapace::protocol::{MAGIC_COOKIE, StunRequest, StunResponse};

/// create a test binding request
fn create_binding_request() -> [u8; 20] {
    let mut data = [0u8; 20];

    // Message Type: Binding Request (0x0001)
    data[0] = 0x00;
    data[1] = 0x01;

    // Message Length: 0
    data[2] = 0x00;
    data[3] = 0x00;

    // Magic Cookie
    data[4..8].copy_from_slice(&MAGIC_COOKIE.to_be_bytes());

    // Transaction ID
    data[8..20].copy_from_slice(b"BENCHMARK123");

    data
}

/// parsing benchmark
fn bench_parsing(c: &mut Criterion) {
    let request_data = create_binding_request();

    let mut group = c.benchmark_group("Parsing");
    group.throughput(Throughput::Elements(1));

    group.bench_function("StunRequest", |b| {
        b.iter(|| {
            let req = StunRequest::parse(black_box(&request_data)).unwrap();
            black_box(req)
        })
    });

    group.finish();
}

/// response creation benchmark
fn bench_response(c: &mut Criterion) {
    let transaction_id = *b"BENCHMARK123";
    let client_addr_v4 = SocketAddrV4::new(Ipv4Addr::new(192, 168, 1, 100), 12345);

    let mut group = c.benchmark_group("Response");
    group.throughput(Throughput::Elements(1));

    group.bench_function("StunResponse", |b| {
        b.iter(|| {
            let response = StunResponse::binding_response(
                black_box(&transaction_id),
                black_box(client_addr_v4),
            );
            black_box(&response);
        })
    });

    group.finish();
}

/// full request-response cycle benchmark
fn bench_full_cycle(c: &mut Criterion) {
    let request_data = create_binding_request();
    let client_addr_v4 = SocketAddrV4::new(Ipv4Addr::new(192, 168, 1, 100), 12345);

    let mut group = c.benchmark_group("FullCycle");
    group.throughput(Throughput::Elements(1));

    group.bench_function("request_response", |b| {
        b.iter(|| {
            let request = StunRequest::parse(black_box(&request_data)).unwrap();

            let response =
                StunResponse::binding_response(request.transaction_id, black_box(client_addr_v4));

            black_box(&response);
        })
    });

    group.finish();
}

criterion_group!(benches, bench_parsing, bench_response, bench_full_cycle);
criterion_main!(benches);
