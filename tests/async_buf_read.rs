// Test the behavior of AsyncRead::poll_read:
//
// - poll_fill_buf:
//   ✔ data must be correct
//   ✔ data must be advanced correctly
//   ✔ at the end of the stream an empty buffer is returned.
//   ✔ errors must be returned
//   ✔ pending must be returned
//
// - create different situations of consume:
//   ✔ consume 0
//   ✔ consume part
//   ✔ consume an entire message at once
//   ✔ more than the buffer size
//
mod common;

use
{
	common            :: { *                                 } ,
	async_io_stream   :: { *                                 } ,
	futures           :: { *, task::noop_waker               } ,
	std               :: { task::{ Poll, Context }, pin::Pin } ,
	pretty_assertions :: { assert_eq                         } ,
	assert_matches    :: { *                                 } ,
	log               :: { *                                 } ,
};



#[ derive( Debug, PartialEq, Eq ) ]
//
enum Output
{
	Data(Vec<u8>),
	Pending,
	Error(io::ErrorKind),
}


fn tester( actions: Vec<Action>, expect: Vec<Output>, consume: Vec<usize>, polled: usize )
{
	let stream = TestStream::new( actions.into() );

	let mut wrapped = IoStream::new( stream );
	let     waker   = noop_waker();
	let mut cx      = Context::from_waker( &waker );

	for (i, out) in expect.iter().enumerate()
	{
		match Pin::new( &mut wrapped ).poll_fill_buf( &mut cx )
		{
			Poll::Ready(Ok (data)) => assert_matches!( out, Output::Data ( exp ) => assert_eq!( data     , &exp[..] ) ),
			Poll::Ready(Err(e   )) => assert_matches!( out, Output::Error( err ) => assert_eq!( &e.kind(), err      ) ),
			Poll::Pending          => assert_eq!     ( out, &Output::Pending                                          ),
		}

		debug!( "consume: {}", consume[i] );
		Pin::new( &mut wrapped ).consume( consume[i] );
	}

	assert_eq!( wrapped.inner().polled(), polled );
}


// Send in an empty buffer.
//
#[ test ] fn consume_zero()
{
	// flexi_logger::Logger::with_str( "trace" ).start().expect( "flexi_logger");

	let actions = vec![ vec![ 1, 2, 3 ].into() ];
	let consume = vec![ 0, 0, 0, 0                ];
	let expect  = vec![ Output::Data(vec![ 1, 2, 3 ]), Output::Data(vec![ 1, 2, 3 ]), Output::Data(vec![ 1, 2, 3 ]), Output::Data(vec![ 1, 2, 3 ]) ];

	tester( actions, expect, consume, 1 );
}


// Send in an empty buffer.
//
#[ test ] fn consume_one()
{
	// flexi_logger::Logger::with_str( "trace" ).start().expect( "flexi_logger");

	let actions = vec![ vec![ 1, 2, 3 ].into() ];
	let consume = vec![ 1, 1, 1, 0                ];
	let expect  = vec![ Output::Data(vec![ 1, 2, 3 ]), Output::Data(vec![ 2, 3 ]), Output::Data(vec![ 3 ]), Output::Data(vec![]) ];

	tester( actions, expect, consume, 2 );
}


// Send in an empty buffer.
//
#[ test ] fn consume_entire_msg()
{
	// flexi_logger::Logger::with_str( "trace" ).start().expect( "flexi_logger");

	let actions = vec![ vec![ 1, 2, 3 ].into(), vec![ 4, 5, 6 ].into() ];
	let consume = vec![ 3, 3, 0 ];
	let expect  = vec![ Output::Data(vec![ 1, 2, 3 ]), Output::Data(vec![ 4, 5, 6 ]), Output::Data(vec![]) ];

	tester( actions, expect, consume, 3 );
}


// Send in an empty buffer.
//
#[ test ] #[ should_panic ] fn over_consume()
{
	// flexi_logger::Logger::with_str( "trace" ).start().expect( "flexi_logger");

	let actions = vec![ vec![ 1, 2, 3 ].into(), vec![ 4, 5, 6 ].into() ];
	let consume = vec![ 4, 3, 0 ];
	let expect  = vec![ Output::Data(vec![ 1, 2, 3 ]), Output::Data(vec![ 4, 5, 6 ]), Output::Data(vec![]) ];

	tester( actions, expect, consume, 3 );
}


// Return error.
//
#[ test ] fn bufread_error()
{
	// flexi_logger::Logger::with_str( "trace" ).start().expect( "flexi_logger");

	let actions = vec![ Action::Error( io::ErrorKind::NotConnected ), vec![ 4, 5, 6 ].into() ];
	let consume = vec![ 0, 3, 0 ];
	let expect  = vec![ Output::Error( io::ErrorKind::NotConnected ), Output::Data(vec![ 4, 5, 6 ]), Output::Data(vec![]) ];

	tester( actions, expect, consume, 3 );
}


// Return error.
//
#[ test ] fn bufread_pending()
{
	// flexi_logger::Logger::with_str( "trace" ).start().expect( "flexi_logger");

	let actions = vec![ Action::Pending, vec![ 4, 5, 6 ].into() ];
	let consume = vec![ 0, 3, 0 ];
	let expect  = vec![ Output::Pending, Output::Data(vec![ 4, 5, 6 ]), Output::Data(vec![]) ];

	tester( actions, expect, consume, 3 );
}

