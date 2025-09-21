use bstr::ByteSlice;
use clap::Parser;
use ibig::IBig;
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicI64, Ordering},
};
use std::thread;

#[derive(Parser, Debug)]
#[command(
    name = "fib-find",
    version,
    about = "並列でフィボナッチに部分文字列が出る最初の項を探す"
)]
struct Args {
    #[arg(short = 't', long = "threads", default_value_t = 16)]
    threads: usize,
    #[arg(short = 'n', long = "needle")]
    needle: String,
    #[arg(short = 'c', long = "chunk", default_value_t = 10_000)]
    chunk: i64,
    #[arg(short = 's', long = "start", default_value_t = 0)]
    start: i64,
}

struct Matrix2x2 {
    a11: IBig,
    a12: IBig,
    a21: IBig,
    a22: IBig,
}

impl Matrix2x2 {
    fn new(a11: IBig, a12: IBig, a21: IBig, a22: IBig) -> Self {
        Matrix2x2 { a11, a12, a21, a22 }
    }
}

fn dot(x: &Matrix2x2, y: &Matrix2x2) -> Matrix2x2 {
    Matrix2x2 {
        a11: x.a11.clone() * &y.a11 + &x.a12 * &y.a21,
        a12: x.a11.clone() * &y.a12 + &x.a12 * &y.a22,
        a21: x.a21.clone() * &y.a11 + &x.a22 * &y.a21,
        a22: x.a21.clone() * &y.a12 + &x.a22 * &y.a22,
    }
}

fn calc_fib_x(mut x: i64) -> (IBig, IBig) {
    let mut matrix: Matrix2x2 =
        Matrix2x2::new(IBig::from(1), IBig::from(0), IBig::from(0), IBig::from(1));
    let mut matrix2: Matrix2x2 =
        Matrix2x2::new(IBig::from(1), IBig::from(1), IBig::from(1), IBig::from(0));
    while x > 0 {
        if x % 2 == 1 {
            matrix = dot(&matrix, &matrix2);
        }
        matrix2 = dot(&matrix2, &matrix2);
        x /= 2;
    }
    (matrix.a11, matrix.a12)
}

fn search_part(
    line: u64,
    is_find: Arc<AtomicBool>,
    x_start: Arc<AtomicI64>,
    found_idx: Arc<AtomicI64>,
    needle: Arc<Vec<u8>>,
    chunk: i64,
) {
    let needle = needle.to_vec();
    while !is_find.load(Ordering::Relaxed) {
        let beg = x_start.fetch_add(chunk, Ordering::Relaxed);
        let end = beg + chunk;

        // 先頭2項をダブルで計算してから逐次更新
        let (mut x, mut y) = calc_fib_x(beg + 1);

        // F_beg+1 を判定
        if y.to_string().as_bytes().contains_str(&needle) {
            is_find.store(true, Ordering::Relaxed);
            let now = found_idx.load(Ordering::Relaxed);
            if now == -1 || now > beg {
                found_idx.store(beg, Ordering::Relaxed);
            }
            print!("\x1b[{};0H\x1b[2K", line);
            println!(
                "Find!!! Fib_{} has {:?}",
                beg,
                std::str::from_utf8(&needle).unwrap_or("?")
            );
            break;
        }
        // F_beg+2 を判定
        if x.to_string().as_bytes().contains_str(&needle) {
            is_find.store(true, Ordering::Relaxed);
            let now = found_idx.load(Ordering::Relaxed);
            if now == -1 || now > beg + 1 {
                found_idx.store(beg + 1, Ordering::Relaxed);
            }
            print!("\x1b[{};0H\x1b[2K", line);
            println!(
                "Find!!! Fib_{} has {:?}",
                beg + 1,
                std::str::from_utf8(&needle).unwrap_or("?")
            );
            break;
        }

        for i in beg + 2..end {
            // 次項へ
            (x, y) = (&x + y, x);

            if x.to_string().as_bytes().contains_str(&needle) {
                is_find.store(true, Ordering::Relaxed);
                let now = found_idx.load(Ordering::Relaxed);
                if now == -1 || now > i + 1 {
                    found_idx.store(i + 1, Ordering::Relaxed);
                }
                print!("\x1b[{};0H\x1b[2K", line);
                println!(
                    "Find!!! Fib_{} has {:?}",
                    i + 1,
                    std::str::from_utf8(&needle).unwrap_or("?")
                );
                break;
            }

            if i % (chunk / 100) == 0 {
                let prog = ((i - beg) / (chunk / 100)) as usize;
                let prog = prog.min(100);
                print!("\x1b[{};0H\x1b[2K", line);
                println!(
                    "[{}{}]({}%, {})",
                    "=".repeat(prog),
                    " ".repeat(100usize - prog),
                    (i - beg) / (chunk / 100),
                    i
                );
            }
            if is_find.load(Ordering::Relaxed) {
                break;
            }
        }
    }
}

fn main() {
    let args = Args::parse();

    let found = Arc::new(AtomicBool::new(false));
    let found_idx = Arc::new(AtomicI64::new(-1));
    let counter = Arc::new(AtomicI64::new(args.start));

    let needle = Arc::new(args.needle.into_bytes());

    println!("\x1b[2J");
    let mut threads = Vec::with_capacity(args.threads);

    for i in 0..args.threads {
        let f = Arc::clone(&found);
        let c = Arc::clone(&counter);
        let idx = Arc::clone(&found_idx);
        let n = Arc::clone(&needle);
        let chunk = args.chunk;

        threads.push(thread::spawn(move || {
            search_part(i as u64 + 1, f, c, idx, n, chunk)
        }));
    }

    for th in threads {
        let _ = th.join();
    }

    println!("\x1b[2J");
    println!("idx:{}", found_idx.load(Ordering::Relaxed));
}
