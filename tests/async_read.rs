// Test the behavior of AsyncRead::poll_read:
//
// ✔ pass empty buffer
//
// - pass buffer exactly one message
//
//    ✔ should only poll stream once,
//    ✔ should exactly return the entire message in each buffer
//    ✔ what if stream returns error
//    - what if stream ends
//
// ✔ pass a buffer smaller than message,
//
//    ✔ should return the rest on subsequent calls
//
// - pass buffer bigger than message
//
//    ✔ should poll stream twice and fill entire buffer
//    ✔ if second poll returns pending, should return data read so far
//    - if second poll returns pending, waker shouldn't be woken up
//    ✔ what if second poll returns error
//
mod common;

use
{
	common            :: { *                                 } ,
	async_io_stream   :: { *                                 } ,
	futures           :: { *, task::noop_waker               } ,
	std               :: { task::{ Poll, Context }, pin::Pin } ,
	pretty_assertions :: { assert_eq                         } ,
	assert_matches    :: { assert_matches                    } ,
	log               :: { *                                 } ,
};



#[ derive( Debug, PartialEq, Eq, Clone, Copy ) ]
//
enum Output
{
	Read(usize),
	Pending,
	Error(io::ErrorKind),
}


fn tester( actions: Vec<Action>, read_out: Vec<Output>, expect: Vec<Vec<u8>>, polled: usize )
{
	tester_futures( actions.clone(), read_out.clone(), expect.clone(), polled );

	#[ cfg( feature = "tokio_io" ) ]
	//
	tester_tokio( actions, read_out, expect, polled );
}


fn tester_futures( actions: Vec<Action>, read_out: Vec<Output>, expect: Vec<Vec<u8>>, polled: usize )
{
	let stream = TestStream::new( actions.into() );

	let mut wrapped = IoStream::new( stream );
	let     waker   = noop_waker();
	let mut cx      = Context::from_waker( &waker );

	for (i, out) in expect.iter().enumerate()
	{
		let mut buf = vec![ 0u8; out.len() ];

		debug!( "buf.len(): {}", buf.len() );

		match Pin::new( &mut wrapped ).poll_read( &mut cx, &mut buf )
		{
			Poll::Ready(Ok (read)) => assert_matches!( read_out[i], Output::Read ( n   ) => assert_eq!( read    , n   ) ),
			Poll::Ready(Err(e   )) => assert_matches!( read_out[i], Output::Error( err ) => assert_eq!( e.kind(), err ) ),
			Poll::Pending          => assert_eq!     ( read_out[i], Output::Pending                                     ),
		}

		assert_eq!( &buf , out );
	}

	assert_eq!( wrapped.inner().polled(), polled );
}


#[ cfg( feature = "tokio_io" ) ]
//
fn tester_tokio( actions: Vec<Action>, read_out: Vec<Output>, expect: Vec<Vec<u8>>, polled: usize )
{
	let stream = TestStream::new( actions.into() );

	let mut wrapped = IoStream::new( stream );
	let     waker   = noop_waker();
	let mut cx      = Context::from_waker( &waker );

	for (i, out) in expect.iter().enumerate()
	{
		let mut buf = vec![ 0u8; out.len() ];
		let mut readbuf = tokio::io::ReadBuf::new( &mut buf );

		let result = tokio::io::AsyncRead::poll_read( Pin::new( &mut wrapped ), &mut cx, &mut readbuf );

		match result
		{
			Poll::Ready( Ok(()) )  => assert_matches!( read_out[i], Output::Read ( n   ) => assert_eq!( readbuf.filled().len(), n   ) ),
			Poll::Ready(Err(e   )) => assert_matches!( read_out[i], Output::Error( err ) => assert_eq!( e.kind()              , err ) ),
			Poll::Pending          => assert_eq!     ( read_out[i], Output::Pending                                                   ),
		}


		assert_eq!( &readbuf.initialized() , out );
	}

	assert_eq!( wrapped.inner().polled(), polled );
}


// Send in an empty buffer.
//
#[ test ] fn empty_buffer()
{
	// flexi_logger::Logger::with_str( "trace" ).start().expect( "flexi_logger");

	let actions   = vec![ vec![ 1, 1 ].into(), vec![ 2, 2 ].into() ];
	let expect    = vec![ vec![]                                   ];
	let read_out  = vec![ Output::Read(0)                          ];

	tester( actions, read_out, expect, 1 );
}


// Send in a buffer with the same size as the message.
//
#[ test ] fn exact()
{
	// flexi_logger::Logger::with_str( "trace" ).start().expect( "flexi_logger");

	let actions   = vec![ vec![ 1, 1 ].into(), vec![ 2, 2 ].into() ];
	let expect    = vec![ vec![ 1, 1 ]       , vec![ 2, 2 ]        ];
	let read_out  = vec![ Output::Read(2)    , Output::Read(2)     ];

	tester( actions, read_out, expect, 2 );
}


// it should poll twice for the first buffer, which is to big, but then when we call poll_read
// again, it shouldn't poll the underlying stream again, since it already knows that it's ended.
//
#[ test ] fn dont_poll_after_stream_end()
{
	// flexi_logger::Logger::with_str( "trace" ).start().expect( "flexi_logger");

	let actions   = vec![ vec![ 1 ].into()                     ];
	let expect    = vec![ vec![ 1, 0 ]       , vec![ 0 ]       ];
	let read_out  = vec![ Output::Read(1)    , Output::Read(0) ];

	tester( actions, read_out, expect, 2 );
}


// Poll beyond the end of the stream. It depends on the stream, but if the stream
// justs returns None, the wrapper will just keep returning Poll::Ready(Ok(0)).
//
#[ test ] fn exact_beyond_end()
{
	// flexi_logger::Logger::with_str( "trace" ).start().expect( "flexi_logger");

	let actions   = vec![ vec![ 1, 1 ].into(), vec![ 2, 2 ].into()                  ];
	let expect    = vec![ vec![ 1, 1 ]       , vec![ 2, 2 ]       , vec![]          ];
	let read_out  = vec![ Output::Read(2)    , Output::Read(2)    , Output::Read(0) ];

	tester( actions, read_out, expect, 3 );
}


// Poll with a small buffer.
//
#[ test ] fn smaller()
{
	// flexi_logger::Logger::with_str( "trace" ).start().expect( "flexi_logger");

	let actions   = vec![ vec![ 1, 1 ].into(), vec![ 2, 2 ].into()                           ];
	let expect    = vec![ vec![1], vec![1]   , vec![2], vec![2]                              ];
	let read_out  = vec![ Output::Read(1), Output::Read(1), Output::Read(1), Output::Read(1) ];

	tester( actions, read_out, expect, 2 );
}


// Poll with a buffer that's bigger than the message, should be entirely filled up.
//
#[ test ] fn bigger()
{
	// flexi_logger::Logger::with_str( "trace" ).start().expect( "flexi_logger");

	let actions   = vec![ vec![ 1, 1 ].into(), vec![ 2, 2 ].into() ];
	let expect    = vec![ vec![ 1, 1, 2]     , vec![ 2, 0, 0]      ];
	let read_out  = vec![ Output::Read(3)    , Output::Read(1)     ];

	// since the second buffer is bigger, we still try to get more data before returning,
	// so it polls the stream 3 times.
	//
	tester( actions, read_out, expect, 3 );
}


// Pass a buffer that fits 4 messages at once.
//
#[ test ] fn very_big()
{
	// flexi_logger::Logger::with_str( "trace" ).start().expect( "flexi_logger");

	let actions   = vec![ vec![1].into(), vec![2, 2].into(), vec![3].into(), vec![4].into() ];
	let expect    = vec![ vec![ 1, 2, 2, 3, 4 ], vec![ 0, 0, 0, 0 ]                         ];
	let read_out  = vec![ Output::Read(5)   , Output::Read(0)                               ];

	tester( actions, read_out, expect, 5 );
}


// Pending should be swallowed.
//
#[ test ] fn bigger_pending()
{
	// flexi_logger::Logger::with_str( "trace" ).start().expect( "flexi_logger");

	// The pending will be consumed by IoStream for the first buffer, as that wasn't full
	// yet. Then we will poll again when poll_read is called again, so from outside
	// we will never observe the pending.
	//
	let actions   = vec![ vec![ 1, 1 ].into(), Action::Pending, vec![ 2, 2 ].into() ];
	let expect    = vec![ vec![ 1, 1, 0]                      , vec![ 2, 2, 0]      ];
	let read_out  = vec![ Output::Read(2)                     , Output::Read(2)     ];

	// Fourth time is because the last buffer isn't full. So it tries to get more.
	//
	tester( actions, read_out, expect, 4 );
}


// Poll error should be returned on subsequent call.
//
#[ test ] fn bigger_error()
{
	// flexi_logger::Logger::with_str( "trace" ).start().expect( "flexi_logger");

	// The error will be buffered by IoStream for the first buffer, as that wasn't full
	// yet. The error should be returned the subsequent call.
	//
	let actions   = vec![ vec![ 1, 1 ].into(), Action::Error( io::ErrorKind::NotConnected ), vec![ 2, 2 ].into() ];
	let expect    = vec![ vec![ 1, 1, 0]     , vec![ 0, 0, 0]                              , vec![ 2, 2, 0]      ];
	let read_out  = vec![ Output::Read(2)    , Output::Error( io::ErrorKind::NotConnected ), Output::Read(2)     ];

	// Fourth time is because the last buffer isn't full. So it tries to get more.
	//
	tester( actions, read_out, expect, 4 );
}


// It return the error.
//
#[ test ] fn error()
{
	// flexi_logger::Logger::with_str( "trace" ).start().expect( "flexi_logger");

	let actions   = vec![ Action::Error( io::ErrorKind::NotConnected ), vec![ 1, 1 ].into() ];
	let expect    = vec![ vec![ 0, 0, 0]                              , vec![ 1, 1 ]        ];
	let read_out  = vec![ Output::Error( io::ErrorKind::NotConnected ), Output::Read(2)     ];

	tester( actions, read_out, expect, 2 );
}


// It return the pending.
//
#[ test ] fn pending()
{
	// flexi_logger::Logger::with_str( "trace" ).start().expect( "flexi_logger");

	let actions   = vec![ Action::Pending, vec![ 2, 2 ].into() ];
	let expect    = vec![ vec![ 0, 0, 0] , vec![ 2, 2 ]        ];
	let read_out  = vec![ Output::Pending, Output::Read(2)     ];

	tester( actions, read_out, expect, 2 );
}
