use num_traits::Float;

/// An iterator of a sequence of evenly spaced floats.
///
/// Iterator element type is `F`.
#[derive(Clone, Debug)]
pub struct Linspace<F> {
    start: F,
    step: F,
    index: usize,
    len: usize,
}

impl<F> Iterator for Linspace<F>
    where F: Float
{
    type Item = F;

    #[inline]
    fn next(&mut self) -> Option<F> {
        if self.index >= self.len {
            None
        } else {
            // Calculate the value just like numpy.linspace does
            let i = self.index;
            self.index += 1;
            Some(self.start + self.step * F::from(i).unwrap())
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let n = self.len - self.index;
        (n, Some(n))
    }
}

impl<F> DoubleEndedIterator for Linspace<F>
    where F: Float,
{
    #[inline]
    fn next_back(&mut self) -> Option<F> {
        if self.index >= self.len {
            None
        } else {
            // Calculate the value just like numpy.linspace does
            self.len -= 1;
            let i = self.len;
            Some(self.start + self.step * F::from(i).unwrap())
        }
    }
}

impl<F> ExactSizeIterator for Linspace<F>
    where Linspace<F>: Iterator
{}

/// Return an iterator of evenly spaced floats.
///
/// The `Linspace` has `n` elements, where the first
/// element is `a` and the last element is `b`.
///
/// Iterator element type is `F`, where `F` must be
/// either `f32` or `f64`.
///
/// ```
/// ```
#[inline]
pub fn linspace<F>(a: F, b: F, n: usize) -> Linspace<F>
    where F: Float
{
    let step = if n > 1 {
        let nf: F = F::from(n).unwrap();
        (b - a) / (nf - F::one())
    } else {
        F::zero()
    };
    Linspace {
        start: a,
        step: step,
        index: 0,
        len: n,
    }
}

#[cfg(test)]
mod tests {
    use super::linspace;

    #[test]
    fn test_linspace() {
        let vector: Vec<f64> = linspace::<f64>(0.0, 1.0, 5).collect();

        assert_eq!(vector.len(), 5);
        assert!(vector[0] - 0.0 < 0.00001);
        assert!(vector[1] - 0.25 < 0.00001);
        assert!(vector[2] - 0.5 < 0.00001);
        assert!(vector[3] - 0.75 < 0.00001);
        assert!(vector[4] - 1.0 < 0.00001);
    }
}