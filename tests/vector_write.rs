// Test:
//
// ✔ should not call start_send if poll_ready returns pending.
// ✔ Should produce one big message from all buffers
// ✔ return the correct amount of bytes written
// ✔ flush the sink
// ✔ all data should be present.
// ✔ return errors from poll_ready and start_send
// ✔ return error from poll_flush on next call
// ✔ return error from poll_flush on next call to poll_write
// - don't wake up waker from sink
//
mod common;

use
{
	common            :: { *                                              } ,
	async_io_stream   :: { *                                              } ,
	futures           :: { *, task::noop_waker                            } ,
	std               :: { task::{ Poll, Context }, pin::Pin, io::IoSlice } ,
	pretty_assertions :: { assert_eq                                      } ,
	assert_matches    :: { assert_matches                                 } ,
	// log            :: { *                                              } ,
};


fn tester( ra: Vec<ReadyAction>, sa: Vec<SendAction>, fa: Vec<FlushAction>, data: Vec<Vec<u8>> )

	-> ( IoStream<TestSink, Vec<u8>>, Poll<io::Result<usize>> )
{
	let     sink = TestSink::new( ra, sa, fa );
	let mut wrap = IoStream::new( sink );

	let mut bufs = Vec::new();

	let length   = data.len();
	let mut refs = data.split_at(0).1;

	for _ in 0..length
	{
		let (first, tail) = refs.split_at( 1 );
		refs = tail;

		bufs.push( IoSlice::new( &first[0] ) );
	}

	let waker  = noop_waker();
	let mut cx = Context::from_waker( &waker );

	let out = Pin::new( &mut wrap ).poll_write_vectored( &mut cx, &bufs );

	(wrap, out)
}


// Return pending from poll_ready.
//
#[ test ] fn poll_ready_pending()
{
	// flexi_logger::Logger::with_str( "trace" ).start().expect( "flexi_logger");

	let ra = vec![ ReadyAction::Pending ];
	let sa = vec![ SendAction::Ok       ];
	let fa = vec![ FlushAction::Ok      ];

	let data = vec![ vec![ 1, 1 ], vec![ 2, 2 ] ];

	let (wrap, out) = tester( ra, sa, fa, data );

	assert_matches!( out, Poll::Pending );

	assert_eq!( wrap.inner().poll_ready , 1 );
	assert_eq!( wrap.inner().start_send , 0 );
	assert_eq!( wrap.inner().poll_flush , 0 );
	assert_eq!( wrap.inner().items.len(), 0 );
}


// Return errors from poll_ready.
//
#[ test ] fn poll_ready_error()
{
	// flexi_logger::Logger::with_str( "trace" ).start().expect( "flexi_logger");

	let ra = vec![ ReadyAction::Error( io::ErrorKind::NotConnected ) ];
	let sa = vec![ SendAction::Ok                                    ];
	let fa = vec![ FlushAction::Ok                                   ];

	let data = vec![ vec![ 1, 1 ], vec![ 2, 2 ] ];

	let (wrap, out) = tester( ra, sa, fa, data );

	assert_matches!( out, Poll::Ready( Err(e) ) => assert_eq!( e.kind(), io::ErrorKind::NotConnected ) );

	assert_eq!( wrap.inner().poll_ready , 1 );
	assert_eq!( wrap.inner().start_send , 0 );
	assert_eq!( wrap.inner().poll_flush , 0 );
	assert_eq!( wrap.inner().items.len(), 0 );
}


// Normal use case, 2 buffers to one write.
//
#[ test ] fn normal_use()
{
	// flexi_logger::Logger::with_str( "trace" ).start().expect( "flexi_logger");

	let ra = vec![ ReadyAction::Ok ];
	let sa = vec![ SendAction::Ok  ];
	let fa = vec![ FlushAction::Ok ];

	let data = vec![ vec![ 1, 1 ], vec![ 2, 2 ] ];

	let (wrap, out) = tester( ra, sa, fa, data );

	assert_matches!( out, Poll::Ready( Ok(n) ) => assert_eq!( n, 4 ) );

	assert_eq!( wrap.inner().poll_ready , 1                  );
	assert_eq!( wrap.inner().start_send , 1                  );
	assert_eq!( wrap.inner().poll_flush , 1                  );
	assert_eq!( wrap.inner().items.len(), 1                  );
	assert_eq!( wrap.inner().items[0]   , vec![ 1, 1, 2, 2 ] );
}


// Return errors from start_send.
//
#[ test ] fn send_error()
{
	// flexi_logger::Logger::with_str( "trace" ).start().expect( "flexi_logger");

	let ra = vec![ ReadyAction::Ok                                  ];
	let sa = vec![ SendAction::Error( io::ErrorKind::NotConnected ) ];
	let fa = vec![ FlushAction::Ok                                  ];

	let data = vec![ vec![ 1, 1 ], vec![ 2, 2 ] ];

	let (wrap, out) = tester( ra, sa, fa, data );

	assert_matches!( out, Poll::Ready( Err(e) ) => assert_eq!( e.kind(), io::ErrorKind::NotConnected ) );

	assert_eq!( wrap.inner().poll_ready , 1 );
	assert_eq!( wrap.inner().start_send , 1 );
	assert_eq!( wrap.inner().poll_flush , 0 );
	assert_eq!( wrap.inner().items.len(), 0 );
}


// Return errors from flush on next poll_write.
//
#[ test ] fn flush_error_return_from_poll_write()
{
	// flexi_logger::Logger::with_str( "trace" ).start().expect( "flexi_logger");

	let ra = vec![ ReadyAction::Ok                                   ];
	let sa = vec![ SendAction::Ok                                    ];
	let fa = vec![ FlushAction::Error( io::ErrorKind::NotConnected ) ];

	let data = vec![ vec![ 1, 1 ], vec![ 2, 2 ] ];

	let (mut wrap, out) = tester( ra, sa, fa, data );

	assert_matches!( out, Poll::Ready( Ok(n) ) => assert_eq!( n, 4 ) );

	assert_eq!( wrap.inner().poll_ready , 1                  );
	assert_eq!( wrap.inner().start_send , 1                  );
	assert_eq!( wrap.inner().poll_flush , 1                  );
	assert_eq!( wrap.inner().items.len(), 1                  );
	assert_eq!( wrap.inner().items[0]   , vec![ 1, 1, 2, 2 ] );

	let waker  = noop_waker();
	let mut cx = Context::from_waker( &waker );

	let out = Pin::new( &mut wrap ).poll_write( &mut cx, &[ 1 ] );

	assert_matches!( out, Poll::Ready( Err(e) ) => assert_eq!( e.kind(), io::ErrorKind::NotConnected ) );
}


// Return errors from flush on next poll_write_vectored.
//
#[ test ] fn flush_error_return_from_poll_write_vectored()
{
	// flexi_logger::Logger::with_str( "trace" ).start().expect( "flexi_logger");

	let ra = vec![ ReadyAction::Ok                                   ];
	let sa = vec![ SendAction::Ok                                    ];
	let fa = vec![ FlushAction::Error( io::ErrorKind::NotConnected ) ];

	let data = vec![ vec![ 1, 1 ], vec![ 2, 2 ] ];

	let (mut wrap, out) = tester( ra, sa, fa, data );

	assert_matches!( out, Poll::Ready( Ok(n) ) => assert_eq!( n, 4 ) );

	assert_eq!( wrap.inner().poll_ready , 1                  );
	assert_eq!( wrap.inner().start_send , 1                  );
	assert_eq!( wrap.inner().poll_flush , 1                  );
	assert_eq!( wrap.inner().items.len(), 1                  );
	assert_eq!( wrap.inner().items[0]   , vec![ 1, 1, 2, 2 ] );

	let waker  = noop_waker();
	let mut cx = Context::from_waker( &waker );

	let out = Pin::new( &mut wrap ).poll_write_vectored( &mut cx, &[ IoSlice::new( &[] ) ] );

	assert_matches!( out, Poll::Ready( Err(e) ) => assert_eq!( e.kind(), io::ErrorKind::NotConnected ) );
}


