use
{
	futures :: { *  } ,
	log     :: { *  } ,
	std     :: { io, task::{ Poll, Context }, pin::Pin, collections::VecDeque } ,
};


#[ derive( Debug, PartialEq, Eq ) ]
//
pub enum Action
{
	Pending                ,
	Error( io::ErrorKind ) ,
	Data ( Vec<u8>   )     ,
}

impl From<Vec<u8>> for Action
{
	fn from( input: Vec<u8> ) -> Self
	{
		Action::Data( input )
	}
}

impl From<io::Error> for Action
{
	fn from( input: io::Error ) -> Self
	{
		Action::Error( input.kind() )
	}
}




pub struct TestStream
{
	actions: VecDeque<Action> ,
	polled : usize            ,
}


impl TestStream
{
	pub fn new( actions: VecDeque<Action> ) -> Self
	{
		Self
		{
			actions     ,
			polled  : 0 ,
		}
	}


	pub fn polled( &self ) -> usize
	{
		self.polled
	}
}


impl Stream for TestStream
{
	type Item = Result< Vec<u8>, io::Error >;

	fn poll_next(mut self: Pin<&mut Self>, _cx: &mut Context) -> Poll<Option<Self::Item>>
	{
		self.polled += 1;

		if let Some( action ) = self.actions.pop_front()
		{
			trace!( "poll_next with: {:?}", &action );

			match action
			{
 				Action::Pending    => Poll::Pending                           ,
 				Action::Data(data) => Poll::Ready( Ok ( data       ).into() ) ,
				Action::Error(err) => Poll::Ready( Err( err.into() ).into() ) ,
			}
		}

		else
		{
			Poll::Ready( None.into() )
		}
	}
}


impl Sink< Vec<u8> > for TestStream
{
	type Error = io::Error;

	fn poll_ready( self: Pin<&mut Self>, _cx: &mut Context ) -> Poll<Result<(), Self::Error>>
	{
		Poll::Ready(Ok(()))
	}


	fn start_send( self: Pin<&mut Self>, _item: Vec<u8> ) -> Result<(), Self::Error>
	{
		Ok(())
	}


	fn poll_flush( self: Pin<&mut Self>, _cx: &mut Context ) -> Poll<Result<(), Self::Error>>
	{
		Poll::Ready(Ok(()))
	}


	fn poll_close( self: Pin<&mut Self>, _cx: &mut Context ) -> Poll<Result<(), Self::Error>>
	{
		Poll::Ready(Ok(()))
	}
}
