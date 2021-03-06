use std::cmp::Eq;
use std::fmt::Display;
use std::iter::FromIterator;

pub trait Parser<T: Display + Eq, E>: Sized {
    fn preview(&self) -> Option<&T>;
    fn consume(&mut self) -> Option<T>;
    fn current_pos(&self) -> (i32, i32);
    fn error<S: Into<String>>(&self, message: S) -> E;

    fn save(&mut self);
    fn load(&mut self);

    fn next(&mut self) -> Result<T, E> {
        self.consume().ok_or(self.error("unexpected eof"))
    }

    fn predicate<F>(&mut self, pred: F) -> Result<T, E>
        where F: Fn(&T) -> bool
    {
        let x = try!(self.next());
        if pred(&x) {
            Ok(x)
        } else {
            Err(self.error(format!("unexpected token {}", x)))
        }
    }

    fn atom(&mut self, expected: T) -> Result<T, E> {
        let x = try!(self.next());
        if x == expected {
            Ok(x)
        } else {
            Err(self.error(format!("unexpected token {}, expected {}", x, expected)))
        }
    }

    fn string<S, O>(&mut self, s: S) -> Result<O, E>
        where S: IntoIterator<Item = T>,
              O: FromIterator<T>
    {
        let mut res: Vec<T> = Vec::new();
        for c in s {
            match self.atom(c) {
                Ok(x) => res.push(x),
                Err(err) => return Err(err),
            }
        }
        Ok(O::from_iter(res))
    }

    fn try<O, F>(&mut self, parser: F) -> Result<O, E>
        where F: Fn(&mut Self) -> Result<O, E>
    {
        self.save();
        parser(self).map_err(|x| {
                                 self.load();
                                 x
                             })
    }

    fn choose<O>(&mut self, parsers: &[&Fn(&mut Self) -> Result<O, E>]) -> Result<O, E> {
        for parser in parsers {
            match self.try(parser) {
                Ok(x) => return Ok(x),
                Err(_) => continue,
            }
        }

        Err(self.error(match self.preview() {
                           Some(x) => format!("unexpected token {}", x),
                           None => String::from("unexpected eof"),
                       }))
    }

    fn many<X, F, O>(&mut self, parser: F) -> Result<O, E>
        where F: Fn(&mut Self) -> Result<X, E>,
              O: FromIterator<X>
    {
        let mut res: Vec<X> = Vec::new();
        loop {
            match self.try(&parser) {
                Ok(x) => res.push(x),
                Err(_) => break,
            }
        }
        Ok(O::from_iter(res))
    }

    fn many1<X, F, O>(&mut self, parser: F) -> Result<O, E>
        where F: Fn(&mut Self) -> Result<X, E>,
              O: FromIterator<X>
    {
        let mut res: Vec<X> = Vec::new();
        res.push(try!(parser(self)));
        loop {
            match self.try(&parser) {
                Ok(x) => res.push(x),
                Err(_) => break,
            }
        }
        Ok(O::from_iter(res))
    }

    fn optional<X, F>(&mut self, parser: F)
        where F: Fn(&mut Self) -> Result<X, E>
    {
        let _ = self.try(parser);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TP {
        input: Vec<i32>,
        cursor: usize,
        saved_cursor: usize,
    }

    impl TP {
        fn new(input: &[i32]) -> TP {
            TP {
                input: input.to_vec(),
                cursor: 0,
                saved_cursor: 0,
            }
        }
    }

    impl Parser<i32, String> for TP {
        fn consume(&mut self) -> Option<i32> {
            match self.input.get(self.cursor) {
                Some(x) => {
                    self.cursor += 1;
                    Some(x.clone())
                }
                None => None,
            }
        }

        fn preview(&self) -> Option<&i32> {
            self.input.get(self.cursor)
        }

        fn current_pos(&self) -> (i32, i32) {
            (0, 0)
        }

        fn error<S: Into<String>>(&self, message: S) -> String {
            message.into()
        }

        fn save(&mut self) {
            self.saved_cursor = self.cursor;
        }

        fn load(&mut self) {
            self.cursor = self.saved_cursor;
        }
    }

    type TPR = Result<Vec<i32>, String>;

    fn err<T>(m: &str) -> Result<T, String> {
        Err(String::from(m))
    }

    #[test]
    fn next_success() {
        let mut p = TP::new(&[1, 2, 3]);
        assert_eq!(p.next(), Ok(1));
        assert_eq!(p.next(), Ok(2));
        assert_eq!(p.next(), Ok(3));
    }

    #[test]
    fn next_fail_empty() {
        let mut p = TP::new(&[]);
        assert_eq!(p.next(), err("unexpected eof"));
    }

    #[test]
    fn predicate_success() {
        let mut p = TP::new(&[2, 4, 6]);
        assert_eq!(p.predicate(|x| x % 2 == 0), Ok(2));
        assert_eq!(p.predicate(|x| x % 2 == 0), Ok(4));
        assert_eq!(p.predicate(|x| x % 2 == 0), Ok(6));
    }

    #[test]
    fn predicate_fail_empty() {
        let mut p = TP::new(&[]);
        assert_eq!(p.predicate(|x| x % 2 == 0), err("unexpected eof"));
    }

    #[test]
    fn predicate_fail_not_satisfy() {
        let mut p = TP::new(&[3, 5, 7]);
        assert_eq!(p.predicate(|x| x % 2 == 0), err("unexpected token 3"));
    }

    #[test]
    fn atom_success() {
        let mut p = TP::new(&[2, 4, 6]);
        assert_eq!(p.atom(2), Ok(2));
        assert_eq!(p.atom(4), Ok(4));
        assert_eq!(p.atom(6), Ok(6));
    }

    #[test]
    fn atom_fail_empty() {
        let mut p = TP::new(&[]);
        assert_eq!(p.atom(2), err("unexpected eof"));
    }

    #[test]
    fn atom_fail_not_expected() {
        let mut p = TP::new(&[3, 5, 7]);
        assert_eq!(p.atom(3), Ok(3));
        assert_eq!(p.atom(4), err("unexpected token 5, expected 4"));
    }

    #[test]
    fn string_success() {
        let mut p = TP::new(&[2, 4, 6]);
        assert_eq!(p.string(vec![2, 4, 6]), Ok(vec![2, 4, 6]));
    }

    #[test]
    fn string_fail_empty() {
        let mut p = TP::new(&[]);
        assert_eq!(p.string(vec![2, 4, 6]) as TPR, err("unexpected eof"));
    }

    #[test]
    fn string_fail_not_expected() {
        let mut p = TP::new(&[2, 5, 6]);
        assert_eq!(p.string(vec![2, 4, 6]) as TPR,
                   err("unexpected token 5, expected 4"));
    }

    #[test]
    fn try_success() {
        let mut p = TP::new(&[2, 4, 6]);
        assert_eq!(p.try(|p| p.string(vec![2, 4, 6])), Ok(vec![2, 4, 6]));
    }

    #[test]
    fn try_fail_recover() {
        let mut p = TP::new(&[2, 4, 6]);
        assert_eq!(p.try(|p| p.string(vec![2, 4, 7])) as TPR,
                   err("unexpected token 6, expected 7"));
        assert_eq!(p.try(|p| p.string(vec![2, 4, 6])), Ok(vec![2, 4, 6]));
    }

    #[test]
    fn choose_success() {
        let mut p = TP::new(&[1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
        assert_eq!(p.choose(&[&|p| p.string(vec![1, 2, 3]),
                              &|p| p.string(vec![4, 5, 6, 7]),
                              &|p| p.string(vec![4, 5, 6])]),
                   Ok(vec![1, 2, 3]));
    }

    #[test]
    fn choose_success_with_recover() {
        let mut p = TP::new(&[4, 5, 6, 7, 8, 9, 10]);
        assert_eq!(p.choose(&[&|p| p.string(vec![1, 2, 3]),
                              &|p| p.string(vec![4, 5, 6, 8]),
                              &|p| p.string(vec![4, 5, 6])]),
                   Ok(vec![4, 5, 6]));
    }

    #[test]
    fn choose_fail_no_match() {
        let mut p = TP::new(&[5, 6, 7, 8, 9, 10]);
        assert_eq!(p.choose(&[&|p| p.string(vec![1, 2, 3]),
                              &|p| p.string(vec![4, 5, 6, 8]),
                              &|p| p.string(vec![4, 5, 6])]) as TPR,
                   err("unexpected token 5"));
    }

    #[test]
    fn choose_fail_empty() {
        let mut p = TP::new(&[]);
        assert_eq!(p.choose(&[&|p| p.string(vec![1, 2, 3]),
                              &|p| p.string(vec![4, 5, 6, 8]),
                              &|p| p.string(vec![4, 5, 6])]) as TPR,
                   err("unexpected eof"));
    }

    #[test]
    fn many_success() {
        let lt5 = |p: &mut TP| -> Result<i32, String> { p.predicate(|x| *x < 5) };

        assert_eq!(TP::new(&[1, 2, 3, 4]).many(&lt5), Ok(vec![1, 2, 3, 4]));
        assert_eq!(TP::new(&[1, 2, 3, 4, 5, 6, 7, 8]).many(&lt5),
                   Ok(vec![1, 2, 3, 4]));
        assert_eq!(TP::new(&[4, 5, 6, 7, 8]).many(&lt5), Ok(vec![4]));
        assert_eq!(TP::new(&[5, 6, 7, 8]).many(&lt5), Ok(vec![]));
    }

    #[test]
    fn many1_success() {
        let lt5 = |p: &mut TP| -> Result<i32, String> { p.predicate(|x| *x < 5) };

        assert_eq!(TP::new(&[1, 2, 3, 4]).many1(&lt5), Ok(vec![1, 2, 3, 4]));
        assert_eq!(TP::new(&[1, 2, 3, 4, 5, 6, 7, 8]).many1(&lt5),
                   Ok(vec![1, 2, 3, 4]));
        assert_eq!(TP::new(&[4, 5, 6, 7, 8]).many1(&lt5), Ok(vec![4]));
    }

    #[test]
    fn many1_fail() {
        let lt5 = |p: &mut TP| -> Result<i32, String> { p.predicate(|x| *x < 5) };

        assert_eq!(TP::new(&[5, 6, 7, 8]).many1(&lt5) as TPR,
                   err("unexpected token 5"));
    }

    #[test]
    fn optional_success() {
        let mut p = TP::new(&[1, 2, 3, 4, 5]);
        p.optional(|p| p.atom(1));
        assert_eq!(p.string(vec![2, 3, 4]), Ok(vec![2, 3, 4]));
    }

    #[test]
    fn optional_success_without_matching() {
        let mut p = TP::new(&[1, 2, 3, 4, 5]);
        p.optional(|p| p.atom(2));
        assert_eq!(p.string(vec![1, 2, 3]), Ok(vec![1, 2, 3]));
    }
}
