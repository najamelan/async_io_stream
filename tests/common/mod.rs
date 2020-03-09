#![ allow( dead_code ) ]

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







#[ derive( Debug, PartialEq, Eq ) ]
//
pub enum ReadyAction
{
	Pending                ,
	Ok                     ,
	Error( io::ErrorKind ) ,
}

#[ derive( Debug, PartialEq, Eq ) ]
//
pub enum SendAction
{
	Ok                     ,
	Error( io::ErrorKind ) ,
}

#[ derive( Debug, PartialEq, Eq ) ]
//
pub enum FlushAction
{
	Pending                ,
	Ok                     ,
	Error( io::ErrorKind ) ,
}


pub struct TestSink
{
	pub poll_ready: usize , // # times poll_ready was called.
	pub start_send: usize ,
	pub poll_flush: usize ,

	pub ready_actions : Vec< ReadyAction > ,
	pub send_actions  : Vec< SendAction  > ,
	pub flush_actions : Vec< FlushAction > ,

	pub items: Vec< Vec<u8> > ,
}


impl TestSink
{
	pub fn new( ready_actions: Vec< ReadyAction>, send_actions: Vec< SendAction>, flush_actions: Vec< FlushAction> ) -> Self
	{
		Self
		{
			poll_ready: 0 ,
			start_send: 0 ,
			poll_flush: 0 ,
			ready_actions ,
			send_actions  ,
			flush_actions ,

			items: Vec::new() ,
		}
	}
}


impl Sink< Vec<u8> > for TestSink
{
	type Error = io::Error;

	fn poll_ready( mut self: Pin<&mut Self>, _cx: &mut Context ) -> Poll<Result<(), Self::Error>>
	{
		self.poll_ready += 1;

		match self.ready_actions[ self.poll_ready - 1 ]
		{
			ReadyAction::Pending  => Poll::Pending                           ,
			ReadyAction::Ok       => Poll::Ready( Ok(())                   ) ,
			ReadyAction::Error(e) => Poll::Ready( Err( io::Error::from(e) )) ,
		}
	}


	fn start_send( mut self: Pin<&mut Self>, item: Vec<u8> ) -> Result<(), Self::Error>
	{
		self.start_send += 1;


		match self.send_actions[ self.start_send - 1 ]
		{
			SendAction::Error(e) => Err( io::Error::from(e) ) ,

			SendAction::Ok =>
			{
				self.items.push( item );
				Ok(())
			}
		}
	}


	fn poll_flush( mut self: Pin<&mut Self>, _cx: &mut Context ) -> Poll<Result<(), Self::Error>>
	{
		self.poll_flush += 1;

		match self.flush_actions[ self.poll_flush - 1 ]
		{
			FlushAction::Pending  => Poll::Pending                           ,
			FlushAction::Ok       => Poll::Ready( Ok(())                   ) ,
			FlushAction::Error(e) => Poll::Ready( Err( io::Error::from(e) )) ,
		}
	}


	fn poll_close( self: Pin<&mut Self>, _cx: &mut Context ) -> Poll<Result<(), Self::Error>>
	{
		Poll::Ready(Ok(()))
	}
}
