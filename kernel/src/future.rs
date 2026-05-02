use core::{
    future::poll_fn,
    pin::{Pin, pin},
    task::{Context, Poll, Waker},
};

const CONTEXT: Context<'_> = Context::from_waker(Waker::noop());

pub fn block_on<F: Future>(f: F) -> F::Output {
    let mut pinned = pin!(f);
    let mut context = CONTEXT;

    loop {
        match pinned.as_mut().poll(&mut context) {
            core::task::Poll::Ready(value) => return value,
            core::task::Poll::Pending => (),
        }
    }
}

pub async fn yield_now() {
    struct YieldNow {
        yielded: bool,
    }

    impl Future for YieldNow {
        type Output = ();

        fn poll(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<()> {
            if self.yielded {
                return Poll::Ready(());
            }

            self.yielded = true;

            Poll::Pending
        }
    }

    YieldNow { yielded: false }.await;
}

pub fn join<A: Future, B: Future>(a: A, b: B) -> impl Future {
    async {
        let futures = (a, b);

        let mut pinned_futures = pin!(futures);

        let mut results = (Poll::Pending, Poll::Pending);

        poll_fn(|cx| {
            if matches!(&results.0, &Poll::Pending) {
                // SAFETY: PINNING PROJECTION
                results.0 = unsafe {
                    pinned_futures
                        .as_mut()
                        .map_unchecked_mut(|fut| &mut fut.0)
                        .poll(cx)
                };
            }

            if matches!(&results.1, &Poll::Pending) {
                // SAFETY: PINNING PROJECTION
                results.1 = unsafe {
                    pinned_futures
                        .as_mut()
                        .map_unchecked_mut(|fut| &mut fut.1)
                        .poll(cx)
                };
            }

            match (&results.0, &results.1) {
                (Poll::Ready(_), Poll::Ready(_)) => Poll::Ready(()),
                _ => Poll::Pending,
            }
        })
        .await;

        match results {
            (Poll::Ready(val_a), Poll::Ready(val_b)) => (val_a, val_b),
            _ => unreachable!("somehow a future was not done?"),
        }
    }
}

#[macro_export]
macro_rules! join {
    ($fut_a:expr, $($futs:tt)+) => {
        $crate::future::join($fut_a, $crate::join!($($futs)+))
    };
    ($fut_b:expr) => {
        $fut_b
    };
}
