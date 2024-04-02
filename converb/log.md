
11:31:08 [INFO] converb: initializing
11:31:08 [TRACE] (2) nih_plug::wrapper::vst3::wrapper: [/Users/andrewthomas/.cargo/git/checkouts/nih-plug-a2d2dc277b128e13/bb27497/src/wrapper/vst3/wrapper.rs:486] Saved state (61 bytes)
11:31:08 [TRACE] (2) nih_plug::wrapper::vst3::wrapper: [/Users/andrewthomas/.cargo/git/checkouts/nih-plug-a2d2dc277b128e13/bb27497/src/wrapper/vst3/wrapper.rs:486] Saved state (61 bytes)
11:31:08 [INFO] converb: initializing
11:32:54 [INFO] converb: initializing
11:32:54 [TRACE] (2) nih_plug::wrapper::vst3::wrapper: [/Users/andrewthomas/.cargo/git/checkouts/nih-plug-a2d2dc277b128e13/bb27497/src/wrapper/vst3/wrapper.rs:486] Saved state (61 bytes)
11:32:54 [TRACE] (2) nih_plug::wrapper::vst3::wrapper: [/Users/andrewthomas/.cargo/git/checkouts/nih-plug-a2d2dc277b128e13/bb27497/src/wrapper/vst3/wrapper.rs:486] Saved state (61 bytes)
11:32:54 [INFO] converb: initializing
11:32:54 [ERROR] assert_no_alloc: Memory allocation of 43 bytes failed from:
   0: backtrace::backtrace::libunwind::trace
             at /Users/andrewthomas/.cargo/registry/src/index.crates.io-6f17d22bba15001f/backtrace-0.3.71/src/backtrace/libunwind.rs:105:5
      backtrace::backtrace::trace_unsynchronized
             at /Users/andrewthomas/.cargo/registry/src/index.crates.io-6f17d22bba15001f/backtrace-0.3.71/src/backtrace/mod.rs:66:5
   1: backtrace::backtrace::trace
             at /Users/andrewthomas/.cargo/registry/src/index.crates.io-6f17d22bba15001f/backtrace-0.3.71/src/backtrace/mod.rs:53:14
   2: backtrace::capture::Backtrace::create
             at /Users/andrewthomas/.cargo/registry/src/index.crates.io-6f17d22bba15001f/backtrace-0.3.71/src/capture.rs:193:9
   3: backtrace::capture::Backtrace::new
             at /Users/andrewthomas/.cargo/registry/src/index.crates.io-6f17d22bba15001f/backtrace-0.3.71/src/capture.rs:158:22
   4: assert_no_alloc::AllocDisabler::check::{{closure}}
             at /Users/andrewthomas/.cargo/git/checkouts/rust-assert-no-alloc-cb7191db54a1fe46/a6fb4f6/src/lib.rs:163:100
   5: assert_no_alloc::permit_alloc
             at /Users/andrewthomas/.cargo/git/checkouts/rust-assert-no-alloc-cb7191db54a1fe46/a6fb4f6/src/lib.rs:113:12
   6: assert_no_alloc::AllocDisabler::check
             at /Users/andrewthomas/.cargo/git/checkouts/rust-assert-no-alloc-cb7191db54a1fe46/a6fb4f6/src/lib.rs:163:5
   7: <assert_no_alloc::AllocDisabler as core::alloc::global::GlobalAlloc>::alloc
             at /Users/andrewthomas/.cargo/git/checkouts/rust-assert-no-alloc-cb7191db54a1fe46/a6fb4f6/src/lib.rs:181:3
   8: __rust_alloc
             at /Users/andrewthomas/.cargo/git/checkouts/nih-plug-a2d2dc277b128e13/bb27497/src/wrapper/util.rs:28:11
   9: alloc::raw_vec::finish_grow
  10: alloc::raw_vec::RawVec<T,A>::grow_amortized
             at /rustc/cc66ad468955717ab92600c770da8c1601a4ff33/library/alloc/src/raw_vec.rs:404:19
      alloc::raw_vec::RawVec<T,A>::reserve::do_reserve_and_handle
             at /rustc/cc66ad468955717ab92600c770da8c1601a4ff33/library/alloc/src/raw_vec.rs:289:28
  11: alloc::raw_vec::RawVec<T,A>::reserve
             at /rustc/cc66ad468955717ab92600c770da8c1601a4ff33/library/alloc/src/raw_vec.rs:293:13
      alloc::vec::Vec<T,A>::reserve
             at /rustc/cc66ad468955717ab92600c770da8c1601a4ff33/library/alloc/src/vec/mod.rs:909:18
      alloc::vec::Vec<T,A>::append_elements
             at /rustc/cc66ad468955717ab92600c770da8c1601a4ff33/library/alloc/src/vec/mod.rs:1941:9
      <alloc::vec::Vec<T,A> as alloc::vec::spec_extend::SpecExtend<&T,core::slice::iter::Iter<T>>>::spec_extend
             at /rustc/cc66ad468955717ab92600c770da8c1601a4ff33/library/alloc/src/vec/spec_extend.rs:55:23
      alloc::vec::Vec<T,A>::extend_from_slice
             at /rustc/cc66ad468955717ab92600c770da8c1601a4ff33/library/alloc/src/vec/mod.rs:2387:9
      alloc::string::String::push_str
             at /rustc/cc66ad468955717ab92600c770da8c1601a4ff33/library/alloc/src/string.rs:903:9
      <alloc::string::String as core::fmt::Write>::write_str
             at /rustc/cc66ad468955717ab92600c770da8c1601a4ff33/library/alloc/src/string.rs:2818:14
      <&mut W as core::fmt::Write>::write_str
             at /rustc/cc66ad468955717ab92600c770da8c1601a4ff33/library/core/src/fmt/mod.rs:199:9
  12: core::fmt::rt::Argument::fmt
             at /rustc/cc66ad468955717ab92600c770da8c1601a4ff33/library/core/src/fmt/rt.rs:138:9
      core::fmt::write
             at /rustc/cc66ad468955717ab92600c770da8c1601a4ff33/library/core/src/fmt/mod.rs:1094:21
  13: core::fmt::Write::write_fmt
             at /rustc/cc66ad468955717ab92600c770da8c1601a4ff33/library/core/src/fmt/mod.rs:192:9
      std::panicking::begin_panic_handler::PanicPayload::fill::{{closure}}
             at /rustc/cc66ad468955717ab92600c770da8c1601a4ff33/library/std/src/panicking.rs:561:30
      core::option::Option<T>::get_or_insert_with
             at /rustc/cc66ad468955717ab92600c770da8c1601a4ff33/library/core/src/option.rs:1666:26
      std::panicking::begin_panic_handler::PanicPayload::fill
             at /rustc/cc66ad468955717ab92600c770da8c1601a4ff33/library/std/src/panicking.rs:559:13
      <std::panicking::begin_panic_handler::PanicPayload as core::panic::BoxMeUp>::get
             at /rustc/cc66ad468955717ab92600c770da8c1601a4ff33/library/std/src/panicking.rs:577:13
  14: std::panicking::rust_panic_with_hook
             at /rustc/cc66ad468955717ab92600c770da8c1601a4ff33/library/std/src/panicking.rs:710:30
  15: std::panicking::begin_panic_handler::{{closure}}
             at /rustc/cc66ad468955717ab92600c770da8c1601a4ff33/library/std/src/panicking.rs:599:13
  16: std::sys_common::backtrace::__rust_end_short_backtrace
             at /rustc/cc66ad468955717ab92600c770da8c1601a4ff33/library/std/src/sys_common/backtrace.rs:170:18
  17: rust_begin_unwind
             at /rustc/cc66ad468955717ab92600c770da8c1601a4ff33/library/std/src/panicking.rs:595:5
  18: core::panicking::panic_fmt
             at /rustc/cc66ad468955717ab92600c770da8c1601a4ff33/library/core/src/panicking.rs:67:14
  19: core::result::unwrap_failed
             at /rustc/cc66ad468955717ab92600c770da8c1601a4ff33/library/core/src/result.rs:1652:5
  20: core::result::Result<T,E>::unwrap
             at /rustc/cc66ad468955717ab92600c770da8c1601a4ff33/library/core/src/result.rs:1077:23
  21: converb::upconv::UPConv<T>::process_block
             at /Users/andrewthomas/dev/diy/convrs/converb/src/upconv.rs:74:9
  22: <converb::Converb as nih_plug::plugin::Plugin>::process
             at /Users/andrewthomas/dev/diy/convrs/converb/src/lib.rs:186:27
  23: <nih_plug::wrapper::vst3::wrapper::Wrapper<P> as vst3_sys::vst::ivstaudioprocessor::IAudioProcessor>::process::{{closure}}
             at /Users/andrewthomas/.cargo/git/checkouts/nih-plug-a2d2dc277b128e13/bb27497/src/wrapper/vst3/wrapper.rs:1401:38
  24: assert_no_alloc::assert_no_alloc
             at /Users/andrewthomas/.cargo/git/checkouts/rust-assert-no-alloc-cb7191db54a1fe46/a6fb4f6/src/lib.rs:82:12
  25: nih_plug::wrapper::util::process_wrapper
             at /Users/andrewthomas/.cargo/git/checkouts/nih-plug-a2d2dc277b128e13/bb27497/src/wrapper/util.rs:186:13
  26: <nih_plug::wrapper::vst3::wrapper::Wrapper<P> as vst3_sys::vst::ivstaudioprocessor::IAudioProcessor>::process
             at /Users/andrewthomas/.cargo/git/checkouts/nih-plug-a2d2dc277b128e13/bb27497/src/wrapper/vst3/wrapper.rs:938:9
  27: <dyn vst3_sys::vst::ivstaudioprocessor::IAudioProcessor as vst3_com::ProductionComInterface<C>>::vtable::iaudioprocessor_process
             at /Users/andrewthomas/.cargo/git/checkouts/vst3-sys-d94cbae274204c7a/b3ff4d7/src/vst/ivstaudioprocessor.rs:61:1
  28: __ZZN6detail16TMemberFuncTypesIM20OPluginProcessorBaseFvPvEE14SProcessorFuncIXadL_ZNS1_11OnMidiEventILi15EEEvS2_EEEEPFvS2_S2_EvENUlS2_S2_E_8__invokeES2_S2_
  29: __ZN7ableton7utility6detail13CallbackTypesI25OMidiAdapterProcessorBaseI20TMxDMidiOutputTraitsEvJEE14CallMemberFuncIXadL_ZNS5_10OnSendMidiEvEEEEvPv
  30: __ZN7ableton7utility6detail13CallbackTypesI25OMidiAdapterProcessorBaseI20TMxDMidiOutputTraitsEvJEE14CallMemberFuncIXadL_ZNS5_10OnSendMidiEvEEEEvPv
  31: __ZN7ableton7utility6detail13CallbackTypesI25OMidiAdapterProcessorBaseI20TMxDMidiOutputTraitsEvJEE14CallMemberFuncIXadL_ZNS5_10OnSendMidiEvEEEEvPv
  32: __ZN7ableton4estd6detail28variant_dispatch_alternativeILm4ENS0_7variantIJNS0_9monostateEN13NZoomScroller13TMouseGestureENS5_13TWheelGestureENS5_13TPinchGestureENS5_15TViewPanGestureEEE26move_assignment_dispatcherEJRSA_SA_EEEDTcldtclsr3stdE7forwardIT0_Efp_E8dispatchIXT_EEspclsr3stdE7forwardIT1_Efp0_EEEOSD_DpOSE_
  33: __ZZN6detail16TMemberFuncTypesIM16OEqualsProcessorFvfEE14SProcessorFuncIXadL_ZNS1_5OnIn1EfEEEEPFvPvfEvENUlS6_fE_8__invokeES6_f
  34: __ZN14OCalcChainBaseI24OAudioBufferWorkingChainL19TWorkingChainPolicy0EE12SOnEndCreateER14OThreadMessage
  35: __ZN14OCalcChainBaseI24OAudioBufferWorkingChainL19TWorkingChainPolicy0EE12SOnEndCreateER14OThreadMessage
  36: __ZN7ableton4estd6detail28variant_dispatch_alternativeILm5ENS0_7variantIJbmNS_7logging8LogLevelENSt3__112basic_stringIcNS6_11char_traitsIcEENS6_9allocatorIcEEEEPNS6_13basic_ostreamIcS9_EENS6_10shared_ptrINS4_9ILogLinesEEEEE27move_constructor_dispatcherEJRSJ_SJ_EEEDTcldtclsr3stdE7forwardIT0_Efp_E8dispatchIXT_EEspclsr3stdE7forwardIT1_Efp0_EEEOSM_DpOSN_
  37: __ZN7ableton4estd6detail28variant_dispatch_alternativeILm5ENS0_7variantIJbmNS_7logging8LogLevelENSt3__112basic_stringIcNS6_11char_traitsIcEENS6_9allocatorIcEEEEPNS6_13basic_ostreamIcS9_EENS6_10shared_ptrINS4_9ILogLinesEEEEE27move_constructor_dispatcherEJRSJ_SJ_EEEDTcldtclsr3stdE7forwardIT0_Efp_E8dispatchIXT_EEspclsr3stdE7forwardIT1_Efp0_EEEOSM_DpOSN_
  38: __pthread_joiner_wake

