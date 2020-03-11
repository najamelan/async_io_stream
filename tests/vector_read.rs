// Test the behavior of AsyncRead::poll_read:
//
// - pass empty buffer
//
// - pass buffer exactly one message
//
//    ✔ should only poll stream once,
//    ✔ should exactly return the entire message in each buffer
//    - what if stream returns error
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
	common       :: { *               } ,
	async_io_stream :: { *               } ,
	// async_std    :: { *               } ,
	futures      :: { *, task::noop_waker } ,
	std          :: { task::{ Poll, Context }, pin::Pin, io::IoSliceMut } ,
	pretty_assertions :: { assert_eq } ,
	assert_matches    :: { * } ,
	// log               :: { * } ,

};



#[ derive( Debug, PartialEq, Eq ) ]
//
enum Output
{
	Read(usize),
	Pending,
	Error(io::ErrorKind),
}


fn tester( actions: Vec<Action>, read_out: Output, expect: Vec<Vec<u8>>, polled: usize )
{
	let stream = TestStream::new( actions.into() );

	let mut wrapped = WsIo::new( stream );
	let     waker   = noop_waker();
	let mut cx      = Context::from_waker( &waker );

	let mut read_vecs = Vec::new();
	let mut read_bufs = Vec::new();

	for out in expect.iter()
	{
		read_vecs.push( vec![ 0u8; out.len() ] );
	}


	let length   = read_vecs.len();
	let mut refs = read_vecs.split_at_mut(0).1;

	for _ in 0..length
	{
		let (first, tail) = refs.split_at_mut( 1 );
		refs = tail;

		read_bufs.push( IoSliceMut::new( &mut first[0] ) );
	}


	match Pin::new( &mut wrapped ).poll_read_vectored( &mut cx, &mut read_bufs )
	{
		Poll::Ready(Ok (read)) => assert_matches!( read_out, Output::Read ( n   ) => assert_eq!( n       , read ) ),
		Poll::Ready(Err(e   )) => assert_matches!( read_out, Output::Error( err ) => assert_eq!( e.kind(), err  ) ),
		Poll::Pending          => assert_eq!     ( read_out, Output::Pending                                      ),
	}

	for (i, out) in expect.iter().enumerate()
	{
		assert_eq!( &read_vecs[i] , out );
	}

	assert_eq!( wrapped.inner().polled(), polled );
}


// Send in an empty buffer. Polls the underlying stream once to be consistent with the default
// behavior from std and futures.
//
#[ test ] fn empty_buffer_vectored()
{
	// flexi_logger::Logger::with_str( "trace" ).start().expect( "flexi_logger");

	let actions   = vec![ vec![ 1, 1 ].into(), vec![ 2, 2 ].into() ];
	let expect    = vec![ vec![]                                   ];
	let read_out  = Output::Read(0);

	tester( actions, read_out, expect, 1 );
}


// Send in a buffer with the same size as the message.
//
#[ test ] fn exact_vectored()
{
	// flexi_logger::Logger::with_str( "trace" ).start().expect( "flexi_logger");

	let actions   = vec![ vec![ 1, 1 ].into(), vec![ 2, 2 ].into() ];
	let expect    = vec![ vec![ 1, 1 ]       , vec![ 2, 2 ]        ];

	tester( actions, Output::Read(4), expect, 2 );
}


// it should poll twice for the first buffer, which is to big, but then when we call poll_read
// again, it shouldn't poll the underlying stream again, since it already knows that it's ended.
//
#[ test ] fn dont_poll_after_stream_end_vectored()
{
	// flexi_logger::Logger::with_str( "trace" ).start().expect( "flexi_logger");

	let actions   = vec![ vec![ 1 ].into()                  ];
	let expect    = vec![ vec![ 1, 0 ]    , vec![ 0 ]       ];

	tester( actions, Output::Read(1), expect, 2 );
}


// Poll beyond the end of the stream. It depends on the stream, but if the stream
// justs returns None, the wrapper will just keep returning Poll::Ready(Ok(0)).
//
#[ test ] fn exact_beyond_end_vectored()
{
	// flexi_logger::Logger::with_str( "trace" ).start().expect( "flexi_logger");

	let actions   = vec![ vec![ 1, 1 ].into(), vec![ 2, 2 ].into()               ];
	let expect    = vec![ vec![ 1, 1 ]       , vec![ 2, 2 ]       , vec![ 0, 0 ] ];

	tester( actions, Output::Read(4), expect, 3 );
}


// Poll with buffers smaller than one message. It should still fill all buffers in
// one call.
//
#[ test ] fn smaller_vectored()
{
	// flexi_logger::Logger::with_str( "trace" ).start().expect( "flexi_logger");

	let actions   = vec![ vec![ 1, 1 ].into(), vec![ 2, 2 ].into() ];
	let expect    = vec![ vec![1], vec![1]   , vec![2], vec![2]    ];

	tester( actions, Output::Read(4), expect, 2 );
}


// Poll with bigger buffers, it should entirely fill up the first buffer with data.
//
#[ test ] fn bigger_vectored()
{
	// flexi_logger::Logger::with_str( "trace" ).start().expect( "flexi_logger");

	let actions   = vec![ vec![ 1, 1 ].into(), vec![ 2, 2 ].into() ];
	let expect    = vec![ vec![ 1, 1, 2]     , vec![ 2, 0, 0]      ];

	// since the second buffer is bigger, we still try to get more data before returning,
	// so it polls the stream 3 times.
	//
	tester( actions, Output::Read(4), expect, 3 );
}


// Pass a buffer that fits 4 messages at once.
//
#[ test ] fn very_big_vectored()
{
	// flexi_logger::Logger::with_str( "trace" ).start().expect( "flexi_logger");

	let actions   = vec![ vec![1].into(), vec![2, 2].into(), vec![3].into(), vec![4].into() ];
	let expect    = vec![ vec![ 1, 2, 2, 3, 4 ], vec![ 0, 0, 0, 0 ]                         ];

	tester( actions, Output::Read(5), expect, 5 );
}


// When the stream is pending, it should just return what it has already read.
//
#[ test ] fn bigger_pending_vectored()
{
	// flexi_logger::Logger::with_str( "trace" ).start().expect( "flexi_logger");

	// The pending will be consumed by WsIo for the first buffer, as that wasn't full
	// yet. Then we will poll again when poll_read is called again, so from outside
	// we will never observe the pending.
	//
	let actions   = vec![ vec![ 1, 1 ].into(), Action::Pending, vec![ 2, 2 ].into() ];
	let expect    = vec![ vec![ 1, 1, 0]                      , vec![ 0, 0, 0]      ];

	tester( actions, Output::Read(2), expect, 2 );
}


// It should buffer the error.
//
#[ test ] fn bigger_error_vectored()
{
	// flexi_logger::Logger::with_str( "trace" ).start().expect( "flexi_logger");

	// The error will be consumed by WsIo for the first buffer, as that wasn't full
	// yet. The error should be returned the subsequent call.
	//
	let actions   = vec![ vec![ 1, 1 ].into(), Action::Error( io::ErrorKind::NotConnected ), vec![ 2, 2 ].into() ];
	let expect    = vec![ vec![ 1, 1, 0]     , vec![ 0, 0, 0]                              , vec![ 0, 0, 0]      ];

	tester( actions, Output::Read(2), expect, 2 );
}


// It return the error.
//
#[ test ] fn error_vectored()
{
	// flexi_logger::Logger::with_str( "trace" ).start().expect( "flexi_logger");


	let actions   = vec![ Action::Error( io::ErrorKind::NotConnected ), vec![ 1, 1 ].into(), vec![ 2, 2 ].into() ];
	let expect    = vec![ vec![ 0, 0, 0]                              , vec![ 0, 0, 0]     , vec![ 0, 0, 0]      ];

	tester( actions, Output::Error( io::ErrorKind::NotConnected ), expect, 1 );
}


// It return the error.
//
#[ test ] fn pending_vectored()
{
	// flexi_logger::Logger::with_str( "trace" ).start().expect( "flexi_logger");


	let actions   = vec![ Action::Pending, vec![ 1, 1 ].into(), vec![ 2, 2 ].into() ];
	let expect    = vec![ vec![ 0, 0, 0] , vec![ 0, 0, 0]     , vec![ 0, 0, 0]      ];

	tester( actions, Output::Pending, expect, 1 );
}
