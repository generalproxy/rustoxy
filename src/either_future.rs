use futures::{Future,Poll};

pub enum EitherFuture<L,R,T,E>
    where L:Future<Item=T,Error=E>,
          R:Future<Item=T,Error=E>
{
    Left(L),
    Right(R)
}

use self::EitherFuture::{Left,Right};

impl<L,R,T,E> Future for EitherFuture<L,R,T,E>
    where L:Future<Item=T,Error=E>,
          R:Future<Item=T,Error=E>
{
    type Item=T;
    type Error=E;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        match self {
            Left(f) => f.poll(),
            Right(f) => f.poll()
        }
    }
}    