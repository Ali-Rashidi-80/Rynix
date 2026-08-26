fn host_mod_pow_mul(acc0: i64, base: i64, n: i64, m: i64) -> i64 {
    debug_assert!(m > 1 && acc0 >= 0 && base > 0 && n >= 0);
    let mulmod = |x: i64, y: i64| -> i64 {
        ((x as i128 * y as i128).rem_euclid(m as i128)) as i64
    };
    let mut r = 1i64;
    let mut b = base % m;
    let mut e = n;
    while e > 0 {
        if e & 1 != 0 {
            r = mulmod(r, b);
        }
        b = mulmod(b, b);
        e >>= 1;
    }
    mulmod(acc0, r)
}

/// Modular inverse via extended Euclid, or `None` if `gcd(a,m) != 1`.
fn host_modinv(mut a: i64, m: i64) -> Option<i64> {
    if m <= 1 {
        return None;
    }
    a = a.rem_euclid(m);
    if a == 0 {
        return None;
    }
    let (mut t, mut newt) = (0i64, 1i64);
    let (mut r, mut newr) = (m, a);
    while newr != 0 {
        let q = r / newr;
        (t, newt) = (newt, t - q * newt);
        (r, newr) = (newr, r - q * newr);
    }
    if r > 1 {
        return None;
    }
    Some(t.rem_euclid(m))
}

fn host_euclid_gcd(mut a: i64, mut b: i64) -> i64 {
    while b != 0 {
        let t = a % b;
        a = b;
        b = t;
    }
    a
}

fn count_primes_inclusive(limit: i64) -> i64 {
    if limit < 2 {
        return 0;
    }
    let n = limit as usize;
    let mut is_prime = vec![true; n + 1];
    is_prime[0] = false;
    is_prime[1] = false;
    let mut p = 2usize;
    while p * p <= n {
        if is_prime[p] {
            let mut m = p * p;
            while m <= n {
                is_prime[m] = false;
                m += p;
            }
        }
        p += 1;
    }
    is_prime[2..=n].iter().filter(|&&x| x).count() as i64
}

