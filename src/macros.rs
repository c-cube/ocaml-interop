// Copyright (c) SimpleStaking and Tezedge Contributors
// SPDX-License-Identifier: MIT

#[cfg(doc)]
use crate::*;

/// Opens a new frame inside of which OCaml values can be rooted to have them tracked by the GC.
///
/// The first argument to this macro must a reference to an OCaml runtime handle.
///
/// The second argument is a list of "root variables" to reserve. These variables can
/// be used to "root" [`OCaml`] values to obtain [`OCamlRooted`] values that can be used
/// to recover stale references to [`OCaml`] values after calls to the OCaml runtime.
///
/// # Examples
///
/// The following example reserves two root variables which are consumed to create two [`OCamlRooted`]
/// values later used to retrieve two OCaml values after performing allocations through the OCaml runtime:
///
/// ```
/// # use ocaml_interop::*;
/// # ocaml! {
/// #    fn print_endline(s: String);
/// # }
/// # fn ocaml_frame_macro_example(cr: &mut OCamlRuntime) {
///     ocaml_frame!(cr, (hello_ocaml, bye_ocaml), {
///         let hello_ocaml = &to_ocaml!(cr, "hello OCaml!", hello_ocaml);
///         let bye_ocaml = &to_ocaml!(cr, "bye OCaml!", bye_ocaml);
///         ocaml_call!(print_endline(cr, cr.get(hello_ocaml)));
///         ocaml_call!(print_endline(cr, cr.get(bye_ocaml)));
///         // Values that don't need to be kept across calls can be used directly
///         let immediate_use = to_ocaml!(cr, "no need to `keep` me");
///         ocaml_call!(print_endline(cr, immediate_use));
///     });
/// # }
/// ```
#[macro_export]
macro_rules! ocaml_frame {
   ($cr:ident, ($($rootvar:ident),+ $(,)?), $body:block) => {{
        let mut frame = $cr.open_frame();
        let local_roots = $crate::repeat_slice!(::core::cell::Cell::new($crate::internal::UNIT), $($rootvar)+);
        let gc = frame.initialize(&local_roots);
        $(
            let $rootvar = unsafe { &mut $crate::internal::OCamlRoot::reserve(gc) };
        )+
        $body
    }};

    ($($t:tt)*) => {
        compile_error!("Invalid `ocaml_frame!` syntax. Must be `ocaml_frame!(cr, (vars, ...), { body-block })`.")
    };
}

/// Declares OCaml functions.
///
/// `ocaml! { pub fn ocaml_name(arg1: Typ1, ...) -> Ret_typ; ... }` declares a function that has been
/// defined in OCaml code and registered with `Callback.register "ocaml_name" the_function`.
///
/// Visibility and return value type can be omitted. The return type defaults to unit when omitted.
///
/// When invoking one of these functions, the first argument must be a `&mut OCamlRuntime`,
/// and the remaining arguments `&OCamlRooted<ArgT>`.
///
/// The return value is an `OCaml<RetType>`.
///
/// # Examples
///
/// ```
/// # use ocaml_interop::*;
/// # struct MyRecord {};
/// ocaml! {
///     // Declares `print_endline`, with a single `String` (`OCaml<String>` when invoked)
///     // argument and unit return type (default when omitted).
///     pub fn print_endline(s: String);
///
///     // Declares `bytes_concat`, with two arguments, an OCaml `bytes` separator,
///     // and an OCaml list of segments to concatenate. Return value is an OCaml `bytes`
///     // value.
///     fn bytes_concat(sep: OCamlBytes, segments: OCamlList<OCamlBytes>) -> OCamlBytes;
/// }
/// ```
#[macro_export]
macro_rules! ocaml {
    () => ();

    ($vis:vis fn $name:ident(
        $arg:ident: $typ:ty $(,)?
    ) $(-> $rtyp:ty)?; $($t:tt)*) => {
        $vis fn $name<'a>(
            cr: &'a mut $crate::OCamlRuntime,
            $arg: &$crate::OCamlRooted<$typ>,
        ) -> Result<$crate::OCaml<'a, $crate::default_to_unit!($($rtyp)?)>, $crate::OCamlError> {
            $crate::ocaml_closure_reference!(F, $name);
            F.call(cr, $arg)
        }

        $crate::ocaml!($($t)*);
    };

    ($vis:vis fn $name:ident(
        $arg1:ident: $typ1:ty,
        $arg2:ident: $typ2:ty $(,)?
    ) $(-> $rtyp:ty)?; $($t:tt)*) => {
        $vis fn $name<'a>(
            cr: &'a mut $crate::OCamlRuntime,
            $arg1: &$crate::OCamlRooted<$typ1>,
            $arg2: &$crate::OCamlRooted<$typ2>,
        ) -> Result<$crate::OCaml<'a, $crate::default_to_unit!($($rtyp)?)>, $crate::OCamlError> {
            $crate::ocaml_closure_reference!(F, $name);
            F.call2(cr, $arg1, $arg2)
        }

        $crate::ocaml!($($t)*);
    };

    ($vis:vis fn $name:ident(
        $arg1:ident: $typ1:ty,
        $arg2:ident: $typ2:ty,
        $arg3:ident: $typ3:ty $(,)?
    ) $(-> $rtyp:ty)?; $($t:tt)*) => {
        $vis fn $name<'a>(
            cr: &'a mut $crate::OCamlRuntime,
            $arg1: &$crate::OCamlRooted<$typ1>,
            $arg2: &$crate::OCamlRooted<$typ2>,
            $arg3: &$crate::OCamlRooted<$typ3>,
        ) -> Result<$crate::OCaml<'a, $crate::default_to_unit!($($rtyp)?)>, $crate::OCamlError> {
            $crate::ocaml_closure_reference!(F, $name);
            F.call3(cr, $arg1, $arg2, $arg3)
        }

        $crate::ocaml!($($t)*);
    };

    ($vis:vis fn $name:ident(
        $($arg:ident: $typ:ty),+ $(,)?
    ) $(-> $rtyp:ty)?; $($t:tt)*) => {
        $vis fn $name<'a>(
            cr: &'a mut $crate::OCamlRuntime,
            $($arg: &$crate::OCamlRooted<$typ>),+
    ) -> Result<$crate::OCaml<'a, $crate::default_to_unit!($($rtyp)?)>, $crate::OCamlError> {
            $crate::ocaml_closure_reference!(F, $name);
            F.call_n(cr, &mut [$($arg.get_raw()),+])
        }

        $crate::ocaml!($($t)*);
    }
}

/// Defines Rust functions callable from OCaml.
///
/// The first argument in these functions declarations is the same as in the [`ocaml_frame!`] macro.
///
/// Arguments and return values must be of type [`OCaml`]`<T>`, or `f64` in the case of unboxed floats.
///
/// The return type defaults to unit when omitted.
///
/// The body of the function has an implicit [`ocaml_frame!`] wrapper, with the lifetimes of every [`OCaml`]`<T>`
/// argument bound to the lifetime of the variable bound to the function's OCaml frame GC handle.
///
/// # Examples
///
/// ```
/// # use ocaml_interop::*;
/// ocaml_export! {
///     fn rust_twice(cr, num: OCamlRooted<OCamlInt>) -> OCaml<OCamlInt> {
///         let num: i64 = num.to_rust(cr);
///         unsafe { OCaml::of_i64_unchecked(num * 2) }
///     }
///
///     fn rust_twice_boxed_i32(cr, num: OCamlRooted<OCamlInt32>) -> OCaml<OCamlInt32> {
///         let num: i32 = num.to_rust(cr);
///         let result = num * 2;
///         ocaml_alloc!(result.to_ocaml(cr))
///     }
///
///     fn rust_add_unboxed_floats_noalloc(_cr, num: f64, num2: f64) -> f64 {
///         num * num2
///     }
///
///     fn rust_twice_boxed_float(cr, num: OCamlRooted<OCamlFloat>) -> OCaml<OCamlFloat> {
///         let num: f64 = num.to_rust(cr);
///         let result = num * 2.0;
///         ocaml_alloc!(result.to_ocaml(cr))
///     }
///
///     fn rust_increment_ints_list(cr, ints: OCamlRooted<OCamlList<OCamlInt>>) -> OCaml<OCamlList<OCamlInt>> {
///         let mut vec: Vec<i64> = ints.to_rust(cr);
///
///         for i in 0..vec.len() {
///             vec[i] += 1;
///         }
///
///         ocaml_alloc!(vec.to_ocaml(cr))
///     }
///
///     fn rust_make_tuple(cr, fst: OCamlRooted<String>, snd: OCamlRooted<OCamlInt>) -> OCaml<(String, OCamlInt)> {
///         let fst: String = fst.to_rust(cr);
///         let snd: i64 = snd.to_rust(cr);
///         let tuple = (fst, snd);
///         ocaml_alloc!(tuple.to_ocaml(cr))
///     }
/// }
/// ```
#[macro_export]
macro_rules! ocaml_export {
    {} => ();

    // Unboxed float return
    {
        fn $name:ident( $cr:ident, $($args:tt)*) -> f64
           $body:block

        $($t:tt)*
    } => {
        $crate::expand_exported_function!(
            @name $name
            @cr $cr
            @roots { }
            @final_args { }
            @proc_args { $($args)*, }
            @return { f64 }
            @body $body
            @original_args $($args)*
        );

        $crate::ocaml_export!{$($t)*}
    };

    // Other (or empty) return value type
    {
        fn $name:ident( $cr:ident, $($args:tt)*) $(-> $rtyp:ty)?
           $body:block

        $($t:tt)*
    } => {
        $crate::expand_exported_function!(
            @name $name
            @cr $cr
            @roots { }
            @final_args { }
            @proc_args { $($args)*, }
            @return { $($rtyp)? }
            @body $body
            @original_args $($args)*
        );

        $crate::ocaml_export!{$($t)*}
    };

    // Invalid arguments

    {
        fn $name:ident( $($invalid_args:tt)* ) $(-> $rtyp:ty)?
           $body:block

        $($t:tt)*
    } => {
        compile_error!("Rust->OCaml exported functions must include an identifier for the OCaml runtime handle followed by at least one argument");
    }
}

/// Calls an OCaml allocator function.
///
/// Useful for calling functions that construct new values and never raise an exception.
///
/// It is used internally by the [`to_ocaml!`] macro, and may be used directly only in rare occasions.
///
/// # Examples
///
/// ```
/// # use ocaml_interop::*;
/// # fn to_ocaml_macro_example(cr: &mut OCamlRuntime) {
///     let hello_string = "hello OCaml!";
///     let ocaml_string: OCaml<String> = ocaml_alloc!(hello_string.to_ocaml(cr));
///     // ...
///     # ()
/// # }
/// ```
#[macro_export]
macro_rules! ocaml_alloc {
    ( $(($obj:expr).)?$($fn:ident).+($cr:ident $(, $($arg:expr),+)? $(,)? ) ) => {
        {
            let res = $(($obj).)?$($fn).+(unsafe { $cr.token() } $(, $($arg),+)? );
            res.mark($cr).eval($cr)
        }
    };

    ( $obj:literal.$($fn:ident).+($cr:ident $(, $($arg:expr),+)? $(,)?) ) => {
        {
            let res = $obj.$($fn).+(unsafe { $cr.token() } $(, $($arg),+)? );
            res.mark($cr).eval($cr)
        }
    };
}

/// Converts Rust values into OCaml values.
///
/// In `to_ocaml!(cr, value)`, `cr` is an OCaml Runtime handle, and `value` is
/// a Rust value of a type that implements the [`ToOCaml`] trait. The resulting
/// value's lifetime is bound to `cr`'s borrow.
///
/// An alternative form accepts a third "root variable" argument: `to_ocaml!(cr, value, rootvar)`.
/// `rootvar` is one of the "root variables" declared when opening an [`ocaml_frame!`].
/// This variant consumes `rootvar` returns an [`OCamlRooted`] value instead of an [`OCaml`] one.
///
/// # Examples
///
/// ```
/// # use ocaml_interop::*;
/// # fn to_ocaml_macro_example(cr: &mut OCamlRuntime) {
///     let ocaml_string: OCaml<String> = to_ocaml!(cr, "hello OCaml!");
///     // ...
///     # ()
/// # }
/// ```
///
/// Variant:
///
/// ```
/// # use ocaml_interop::*;
/// # fn to_ocaml_macro_example(cr: &mut OCamlRuntime) {
///     ocaml_frame!(cr, (rootvar), {
///         let ocaml_string_ref: &OCamlRooted<String> = &to_ocaml!(cr, "hello OCaml!", rootvar);
///         // ...
///         # ()
///     });
/// # }
/// ```
#[macro_export]
macro_rules! to_ocaml {
    ($cr:ident, $obj:expr, $rootvar:ident) => {
        $rootvar.keep($crate::to_ocaml!($cr, $obj))
    };

    ($cr:ident, $obj:expr) => {
        $crate::ocaml_alloc!(($obj).to_ocaml($cr))
    };

    ($($t:tt)*) => {
        compile_error!("Incorrect `to_ocaml!` syntax. Must be `to_ocaml!(cr, expr[, rootvar])`")
    };
}

/// Calls an OCaml function
///
/// The called function must be declared with [`ocaml!`]. The first
/// argument to the function in this invocation must be a `&mut` reference
/// to the OCaml runtime handle.
///
/// The result is either `Ok(result)` or `Err(ocaml_exception)` if
/// an exception is raised by the OCaml function.
///
/// # Examples
///
/// ```
/// # use ocaml_interop::*;
/// ocaml! { fn print_endline(s: String); }
///
/// # fn ocaml_frame_macro_example(cr: &mut OCamlRuntime) {
/// // ...somewhere else inside a function
/// let ocaml_string = to_ocaml!(cr, "hello OCaml!");
/// ocaml_call!(print_endline(cr, ocaml_string)).unwrap();
/// # }
/// ```
#[macro_export]
macro_rules! ocaml_call {
    ( $(($obj:expr).)?$($fn:ident).+($cr:ident, $($arg:expr),+ $(,)?)) => {
        {
            let res = unsafe { $(($obj).)?$($fn).+($cr.token(), $($arg),* ) };
            $crate::gcmark_result!($cr, res)
        }
    };

    ( $($path:ident)::+($cr:ident, $($args:expr),+ $(,)?) ) => {
        {
            let res = unsafe { $($path)::+($cr.token(), $($args),+) };
            $crate::gcmark_result!($cr, res)
        }
    };

    ( $($path:ident)::+.$($field:ident).+($cr:ident, $($args:expr),+ $(,)?) ) => {
        {
            let res = unsafe { $($path)::+$($field).+($cr.token(), $($args),+) };
            $crate::gcmark_result!($cr, res)
        }
    };
}

/// Implements conversion between a Rust struct and an OCaml record.
///
/// See the [`impl_to_ocaml_record!`] and [`impl_from_ocaml_record!`] macros
/// for more details.
#[macro_export]
macro_rules! impl_conv_ocaml_record {
    ($rust_typ:ident => $ocaml_typ:ident {
        $($field:ident : $ocaml_field_typ:ty $(=> $conv_expr:expr)?),+ $(,)?
    }) => {
        $crate::impl_to_ocaml_record! {
            $rust_typ => $ocaml_typ {
                $($field : $ocaml_field_typ $(=> $conv_expr)?),+
            }
        }

        $crate::impl_from_ocaml_record! {
            $ocaml_typ => $rust_typ {
                $($field : $ocaml_field_typ),+
            }
        }
    };

    ($both_typ:ident {
        $($t:tt)*
    }) => {
        $crate::impl_conv_ocaml_record! {
            $both_typ => $both_typ {
                $($t)*
            }
        }
    };
}

/// Implements conversion between a Rust enum and an OCaml variant.
///
/// See the [`impl_to_ocaml_variant!`] and [`impl_from_ocaml_variant!`] macros
/// for more details.
#[macro_export]
macro_rules! impl_conv_ocaml_variant {
    ($rust_typ:ty => $ocaml_typ:ty {
        $($($tag:ident)::+ $(($($slot_name:ident: $slot_typ:ty),+ $(,)?))? $(=> $conv:expr)?),+ $(,)?
    }) => {
        $crate::impl_to_ocaml_variant! {
            $rust_typ => $ocaml_typ {
                $($($tag)::+ $(($($slot_name: $slot_typ),+))? $(=> $conv)?),+
            }
        }

        $crate::impl_from_ocaml_variant! {
            $ocaml_typ => $rust_typ {
                $($($tag)::+ $(($($slot_name: $slot_typ),+))?),+
            }
        }
    };

    ($both_typ:ty {
        $($t:tt)*
    }) => {
        $crate::impl_conv_ocaml_variant!{
            $both_typ => $both_typ {
                $($t)*
            }
        }
    };
}

/// Unpacks an OCaml record into a Rust record
///
/// It is important that the order of the fields remains the same as in the OCaml type declaration.
///
/// # Examples
///
/// ```
/// # use ocaml_interop::*;
/// # ocaml! { fn make_mystruct(unit: ()) -> MyStruct; }
/// struct MyStruct {
///     int_field: i64,
///     string_field: String,
/// }
///
/// // Assuming an OCaml record declaration like:
/// //
/// //      type my_struct = {
/// //          int_field: int;
/// //          string_field: string;
/// //      }
/// //
/// // NOTE: What is important is the order of the fields, not their names.
///
/// # fn unpack_record_example(cr: &mut OCamlRuntime) {
/// let ocaml_struct = ocaml_call!(make_mystruct(cr, OCaml::unit())).unwrap();
/// let my_struct = ocaml_unpack_record! {
///     //  value    => RustConstructor { field: OCamlType, ... }
///     ocaml_struct => MyStruct {
///         int_field: OCamlInt,
///         string_field: String,
///     }
/// };
/// // ...
/// # ()
/// # }
/// ```
#[macro_export]
macro_rules! ocaml_unpack_record {
    ($var:ident => $cons:ident {
        $($field:ident : $ocaml_typ:ty),+ $(,)?
    }) => {{
        let record = $var;
        unsafe {
            let mut current = 0;

            $(
                let $field = record.field::<$ocaml_typ>(current).to_rust();
                current += 1;
            )+

            $cons {
                $($field),+
            }
        }
    }};
}

/// Allocates an OCaml memory block tagged with the specified value.
///
/// It is used internally to allocate OCaml variants, its direct use is
/// not recommended.
#[macro_export]
macro_rules! ocaml_alloc_tagged_block {
    ($cr:ident, $tag:expr, $($field:ident : $ocaml_typ:ty),+ $(,)?) => {
        unsafe {
            $crate::ocaml_frame!($cr, (block), {
                let mut current = 0;
                let field_count = $crate::count_fields!($($field)*);
                let block: $crate::OCamlRooted<()> = block.keep_raw($crate::internal::caml_alloc(field_count, $tag));
                $(
                    let $field: $crate::OCaml<$ocaml_typ> = $crate::to_ocaml!($cr, $field);
                    $crate::internal::store_field(block.get_raw(), current, $field.raw());
                    current += 1;
                )+
                $crate::OCamlAllocResult::of(block.get_raw())
            })
        }
    };
}

/// Allocates an OCaml record built from a Rust record
///
/// Most of the time the [`impl_to_ocaml_record!`] macro will be used to define how records
/// should be converted. This macro is useful when implementing OCaml allocation
/// functions directly.
///
/// It is important that the order of the fields remains the same as in the OCaml type declaration.
///
/// # Examples
///
/// ```
/// # use ocaml_interop::*;
/// struct MyStruct {
///     int_field: u8,
///     string_field: String,
/// }
///
/// // Assuming an OCaml record declaration like:
/// //
/// //      type my_struct = {
/// //          int_field: int;
/// //          string_field: string;
/// //      }
/// //
/// // NOTE: What is important is the order of the fields, not their names.
///
/// # fn alloc_record_example(cr: &mut OCamlRuntime) {
/// let ms = MyStruct { int_field: 132, string_field: "blah".to_owned() };
/// let ocaml_ms: OCamlAllocResult<MyStruct> = ocaml_alloc_record! {
///     //  value { field: OCamlType, ... }
///     cr, ms {  // cr: &mut OCamlRuntime
///         // optionally `=> expr` can be used to pre-process the field value
///         // before the conversion into OCaml takes place.
///         // Inside the expression, a variable with the same name as the field
///         // is bound to a reference to the field value.
///         int_field: OCamlInt => { *int_field as i64 },
///         string_field: String,
///     }
/// };
/// // ...
/// # ()
/// # }
/// ```
#[macro_export]
macro_rules! ocaml_alloc_record {
    ($cr:ident, $self:ident {
        $($field:ident : $ocaml_typ:ty $(=> $conv_expr:expr)?),+ $(,)?
    }) => {
        unsafe {
            $crate::ocaml_frame!($cr, (record), {
                let mut current = 0;
                let field_count = $crate::count_fields!($($field)*);
                let record: $crate::OCamlRooted<()> = record.keep_raw($crate::internal::caml_alloc(field_count, 0));
                $(
                    let $field = &$crate::prepare_field_for_mapping!($self.$field $(=> $conv_expr)?);
                    let $field: $crate::OCaml<$ocaml_typ> = $crate::to_ocaml!($cr, $field);
                    $crate::internal::store_field(record.get_raw(), current, $field.raw());
                    current += 1;
                )+
                $crate::OCamlAllocResult::of(record.get_raw())
            })
        }
    };
}

/// Implements [`FromOCaml`] for mapping an OCaml record into a Rust record.
///
/// It is important that the order of the fields remains the same as in the OCaml type declaration.
///
/// # Examples
///
/// ```
/// # use ocaml_interop::*;
/// # ocaml! { fn make_mystruct(unit: ()) -> MyStruct; }
/// struct MyStruct {
///     int_field: i64,
///     string_field: String,
/// }
///
/// // Assuming an OCaml record declaration like:
/// //
/// //      type my_struct = {
/// //          int_field: int;
/// //          string_field: string;
/// //      }
/// //
/// // NOTE: What is important is the order of the fields, not their names.
///
/// impl_from_ocaml_record! {
///     // Optionally, if Rust and OCaml types don't match:
///     // OCamlType => RustType { ... }
///     MyStruct {
///         int_field: OCamlInt,
///         string_field: String,
///     }
/// }
/// ```
#[macro_export]
macro_rules! impl_from_ocaml_record {
    ($ocaml_typ:ident => $rust_typ:ident {
        $($field:ident : $ocaml_field_typ:ty),+ $(,)?
    }) => {
        unsafe impl $crate::FromOCaml<$ocaml_typ> for $rust_typ {
            fn from_ocaml(v: $crate::OCaml<$ocaml_typ>) -> Self {
                $crate::ocaml_unpack_record! { v =>
                    $rust_typ {
                        $($field : $ocaml_field_typ),+
                    }
                }
            }
        }
    };

    ($both_typ:ident {
        $($t:tt)*
    }) => {
        $crate::impl_from_ocaml_record! {
            $both_typ => $both_typ {
                $($t)*
            }
        }
    };
}

/// Implements [`ToOCaml`] for mapping a Rust record into an OCaml record.
///
/// It is important that the order of the fields remains the same as in the OCaml type declaration.
///
/// # Examples
///
/// ```
/// # use ocaml_interop::*;
/// struct MyStruct {
///     int_field: u8,
///     string_field: String,
/// }
///
/// // Assuming an OCaml record declaration like:
/// //
/// //      type my_struct = {
/// //          int_field: int;
/// //          string_field: string;
/// //      }
/// //
/// // NOTE: What is important is the order of the fields, not their names.
///
/// impl_to_ocaml_record! {
///     // Optionally, if Rust and OCaml types don't match:
///     // RustType => OCamlType { ... }
///     MyStruct {
///         // optionally `=> expr` can be used to preprocess the field value
///         // before the conversion into OCaml takes place.
///         // Inside the expression, a variable with the same name as the field
///         // is bound to a reference to the field value.
///         int_field: OCamlInt => { *int_field as i64 },
///         string_field: String,
///     }
/// }
/// ```
#[macro_export]
macro_rules! impl_to_ocaml_record {
    ($rust_typ:ty => $ocaml_typ:ident {
        $($field:ident : $ocaml_field_typ:ty $(=> $conv_expr:expr)?),+ $(,)?
    }) => {
        unsafe impl $crate::ToOCaml<$ocaml_typ> for $rust_typ {
            fn to_ocaml(&self, token: $crate::OCamlAllocToken) -> $crate::OCamlAllocResult<$ocaml_typ> {
                let cr = unsafe { &mut token.recover_runtime_handle() };
                $crate::ocaml_alloc_record! {
                    cr, self {
                        $($field : $ocaml_field_typ $(=> $conv_expr)?),+
                    }
                }
            }
        }
    };

    ($both_typ:ident {
        $($t:tt)*
    }) => {
        $crate::impl_to_ocaml_record! {
            $both_typ => $both_typ {
                $($t)*
            }
        }
    };
}

/// Implements [`FromOCaml`] for mapping an OCaml variant into a Rust enum.
///
/// It is important that the order of the fields remains the same as in the OCaml type declaration.
///
/// # Examples
///
/// ```
/// # use ocaml_interop::*;
/// enum Movement {
///     StepLeft,
///     StepRight,
///     Rotate(f64),
/// }
///
/// // Assuming an OCaml type declaration like:
/// //
/// //      type movement =
/// //        | StepLeft
/// //        | StepRight
/// //        | Rotate of float
/// //
/// // NOTE: What is important is the order of the tags, not their names.
///
/// impl_from_ocaml_variant! {
///     // Optionally, if Rust and OCaml types don't match:
///     // OCamlType => RustType { ... }
///     Movement {
///         // Alternative: StepLeft  => Movement::StepLeft
///         //              <anyname> => <build-expr>
///         Movement::StepLeft,
///         Movement::StepRight,
///         // Tag field names are mandatory
///         Movement::Rotate(rotation: OCamlFloat),
///     }
/// }
/// ```
#[macro_export]
macro_rules! impl_from_ocaml_variant {
    ($ocaml_typ:ty => $rust_typ:ty {
        $($t:tt)*
    }) => {
        unsafe impl $crate::FromOCaml<$ocaml_typ> for $rust_typ {
            fn from_ocaml(v: $crate::OCaml<$ocaml_typ>) -> Self {
                let result = $crate::ocaml_unpack_variant! {
                    v => {
                        $($t)*
                    }
                };

                let msg = concat!(
                    "Failure when unpacking an OCaml<", stringify!($ocaml_typ), "> variant into ",
                    stringify!($rust_typ), " (unexpected tag value)");

                result.expect(msg)
            }
        }
    };

    ($both_typ:ty {
        $($t:tt)*
    }) => {
        $crate::impl_from_ocaml_variant!{
            $both_typ => $both_typ {
                $($t)*
            }
        }
    };
}

/// Unpacks an OCaml variant and maps it into a Rust enum.
///
/// It is important that the order of the fields remains the same as in the OCaml type declaration.
///
/// # Note
///
/// Unlike with [`ocaml_unpack_record!`], the result of [`ocaml_unpack_variant!`] is a `Result` value.
/// An error will be returned in the case of an unexpected tag value. This may change in the future.
///
/// # Examples
///
/// ```
/// # use ocaml_interop::*;
/// # ocaml! { fn make_ocaml_movement(unit: ()) -> Movement; }
/// enum Movement {
///     StepLeft,
///     StepRight,
///     Rotate(f64),
/// }
///
/// // Assuming an OCaml type declaration like:
/// //
/// //      type movement =
/// //        | StepLeft
/// //        | StepRight
/// //        | Rotate of float
/// //
/// // NOTE: What is important is the order of the tags, not their names.
///
/// # fn unpack_variant_example(cr: &mut OCamlRuntime) {
/// let ocaml_variant = ocaml_call!(make_ocaml_movement(cr, OCaml::unit())).unwrap();
/// let result = ocaml_unpack_variant! {
///     ocaml_variant => {
///         // Alternative: StepLeft  => Movement::StepLeft
///         //              <anyname> => <build-expr>
///         Movement::StepLeft,
///         Movement::StepRight,
///         // Tag field names are mandatory
///         Movement::Rotate(rotation: OCamlFloat),
///     }
/// }.unwrap();
/// // ...
/// # }
#[macro_export]
macro_rules! ocaml_unpack_variant {
    ($self:ident => {
        $($($tag:ident)::+ $(($($slot_name:ident: $slot_typ:ty),+ $(,)?))? $(=> $conv:expr)?),+ $(,)?
    }) => {
        (|| {
            let mut current_block_tag = 0;
            let mut current_long_tag = 0;

            $(
                $crate::unpack_variant_tag!(
                    $self, current_block_tag, current_long_tag,
                    $($tag)::+ $(($($slot_name: $slot_typ),+))? $(=> $conv)?);
            )+

            Err("Invalid tag value found when converting from an OCaml variant")
        })()
    };
}

/// Allocates an OCaml variant, mapped from a Rust enum.
///
/// The match in this conversion is exhaustive, and requires that every enum case is covered.
///
/// It is important that the order of the fields remains the same as in the OCaml type declaration.
///
/// # Examples
///
/// ```
/// # use ocaml_interop::*;
/// # ocaml! { fn make_ocaml_movement(unit: ()) -> Movement; }
/// enum Movement {
///     StepLeft,
///     StepRight,
///     Rotate(f64),
/// }
///
/// // Assuming an OCaml type declaration like:
/// //
/// //      type movement =
/// //        | StepLeft
/// //        | StepRight
/// //        | Rotate of float
/// //
/// // NOTE: What is important is the order of the tags, not their names.
///
/// # fn alloc_variant_example(cr: &mut OCamlRuntime) {
/// let movement = Movement::Rotate(180.0);
/// let ocaml_movement: OCamlAllocResult<Movement> = ocaml_alloc_variant! {
///     cr, movement => {
///         Movement::StepLeft,
///         Movement::StepRight,
///         // Tag field names are mandatory
///         Movement::Rotate(rotation: OCamlFloat),
///     }
/// };
/// // ...
/// # }
/// ```
#[macro_export]
macro_rules! ocaml_alloc_variant {
    ($cr:ident, $self:ident => {
        $($($tag:ident)::+ $(($($slot_name:ident: $slot_typ:ty),+ $(,)?))? $(,)?),+
    }) => {
        $crate::ocaml_alloc_variant_match!{
            $cr, $self, 0u8, 0u8,

            @units {}
            @blocks {}

            @pending $({ $($tag)::+ $(($($slot_name: $slot_typ),+))? })+
        }
    };
}

/// Implements [`ToOCaml`] for mapping a Rust enum into an OCaml variant.
///
/// The match in this conversion is exhaustive, and requires that every enum case is covered.
///
/// It is important that the order of the fields remains the same as in the OCaml type declaration.
///
/// # Examples
///
/// ```
/// # use ocaml_interop::*;
/// enum Movement {
///     StepLeft,
///     StepRight,
///     Rotate(f64),
/// }
///
/// // Assuming an OCaml type declaration like:
/// //
/// //      type movement =
/// //        | StepLeft
/// //        | StepRight
/// //        | Rotate of float
/// //
/// // NOTE: What is important is the order of the tags, not their names.
///
/// impl_to_ocaml_variant! {
///     // Optionally, if Rust and OCaml types don't match:
///     // RustType => OCamlType { ... }
///     Movement {
///         Movement::StepLeft,
///         Movement::StepRight,
///         // Tag field names are mandatory
///         Movement::Rotate(rotation: OCamlFloat),
///     }
/// }
/// ```
#[macro_export]
macro_rules! impl_to_ocaml_variant {
    ($rust_typ:ty => $ocaml_typ:ty {
        $($t:tt)*
    }) => {
        unsafe impl $crate::ToOCaml<$ocaml_typ> for $rust_typ {
            fn to_ocaml(&self, token: $crate::OCamlAllocToken) -> $crate::OCamlAllocResult<$ocaml_typ> {
                let cr = unsafe { &mut token.recover_runtime_handle() };
                $crate::ocaml_alloc_variant! {
                    cr, self => {
                        $($t)*
                    }
                }
            }
        }
    };

    ($both_typ:ty {
        $($t:tt)*
    }) => {
        $crate::impl_to_ocaml_variant!{
            $both_typ => $both_typ {
                $($t)*
            }
        }
    };
}

/// Implements [`FromOCaml`] for mapping an OCaml variant into a Rust enum.
///
/// It is important that the order of the fields remains the same as in the OCaml type declaration.
///
/// # Examples
///
/// ```
/// # use ocaml_interop::*;
/// enum Movement {
///     StepLeft,
///     StepRight,
///     Rotate(f64),
/// }
///
/// // Assuming an OCaml type declaration like:
/// //
/// //      type movement = [
/// //        | `StepLeft
/// //        | `StepRight
/// //        | `Rotate of float
/// //      ]
///
/// impl_from_ocaml_polymorphic_variant! {
///     // Optionally, if Rust and OCaml types don't match:
///     // OCamlType => RustType { ... }
///     Movement {
///         StepLeft  => Movement::StepLeft,
///         StepRight => Movement::StepRight,
///         // Tag field names are mandatory
///         Rotate(rotation: OCamlFloat)
///                   => Movement::Rotate(rotation),
///     }
/// }
/// ```
#[macro_export]
macro_rules! impl_from_ocaml_polymorphic_variant {
    ($ocaml_typ:ty => $rust_typ:ty {
        $($t:tt)*
    }) => {
        unsafe impl $crate::FromOCaml<$ocaml_typ> for $rust_typ {
            fn from_ocaml(v: $crate::OCaml<$ocaml_typ>) -> Self {
                let result = $crate::ocaml_unpack_polymorphic_variant! {
                    v => {
                        $($t)*
                    }
                };

                let msg = concat!(
                    "Failure when unpacking an OCaml<", stringify!($ocaml_typ), "> polymorphic variant into ",
                    stringify!($rust_typ), " (unexpected tag value)");

                result.expect(msg)
            }
        }
    };

    ($both_typ:ty {
        $($t:tt)*
    }) => {
        $crate::impl_from_ocaml_polymorphic_variant!{
            $both_typ => $both_typ {
                $($t)*
            }
        }
    };
}

/// Unpacks an OCaml polymorphic variant and maps it into a Rust enum.
///
/// # Note
///
/// Unlike with [`ocaml_unpack_record!`], the result of [`ocaml_unpack_polymorphic_variant!`] is a `Result` value.
/// An error will be returned in the case of an unexpected tag value. This may change in the future.
///
/// # Examples
///
/// ```
/// # use ocaml_interop::*;
/// # ocaml! { fn make_ocaml_polymorphic_movement(unit: ()) -> Movement; }
/// enum Movement {
///     StepLeft,
///     StepRight,
///     Rotate(f64),
/// }
///
/// // Assuming an OCaml type declaration like:
/// //
/// //      type movement = [
/// //        | `StepLeft
/// //        | `StepRight
/// //        | `Rotate of float
/// //      ]
///
/// # fn unpack_polymorphic_variant_example(cr: &mut OCamlRuntime) {
/// let ocaml_polymorphic_variant = ocaml_call!(make_ocaml_polymorphic_movement(cr, OCaml::unit())).unwrap();
/// let result = ocaml_unpack_polymorphic_variant! {
///     ocaml_polymorphic_variant => {
///         StepLeft  => Movement::StepLeft,
///         StepRight => Movement::StepRight,
///         // Tag field names are mandatory
///         Rotate(rotation: OCamlFloat)
///                   => Movement::Rotate(rotation),
///     }
/// }.unwrap();
/// // ...
/// # }
#[macro_export]
macro_rules! ocaml_unpack_polymorphic_variant {
    ($self:ident => {
        $($tag:ident $(($($slot_name:ident: $slot_typ:ty),+ $(,)?))? => $conv:expr),+ $(,)?
    }) => {
        (|| {
            $(
                $crate::unpack_polymorphic_variant_tag!(
                    $self, $tag $(($($slot_name: $slot_typ),+))? => $conv);
            )+

            Err("Invalid tag value found when converting from an OCaml polymorphic variant")
        })()
    };
}

// Internal utility macros

#[doc(hidden)]
#[macro_export]
macro_rules! repeat_slice {
    (@expr $value:expr;
     @accum [$($accum:expr),+];
     @rest) => {
         [$($accum),+]
     };

    (@expr $value:expr;
     @accum [$($accum:expr),+];
     @rest $_v1:ident $_v2:ident $_v3:ident $_v4:ident $_v5:ident $($vars:ident)*) => {
        $crate::repeat_slice!(
            @expr $value;
            @accum [$value, $value, $value, $value, $value, $($accum),+];
            @rest $($vars)*)
    };

    (@expr $value:expr;
        @accum [$($accum:expr),+];
        @rest $_v1:ident $($vars:ident)*) => {

        $crate::repeat_slice!(
            @expr $value;
            @accum [$value, $($accum),+];
            @rest $($vars)*)
    };

    ($value:expr, $field:ident $($vars:ident)*) => {
        $crate::repeat_slice!(
            @expr $value;
            @accum [$value];
            @rest $($vars)*)
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! count_fields {
    () => {0usize};
    ($_f1:ident $_f2:ident $_f3:ident $_f4:ident $_f5:ident $($fields:ident)*) => {
        5usize + $crate::count_fields!($($fields)*)
    };
    ($field:ident $($fields:ident)*) => {1usize + $crate::count_fields!($($fields)*)};
}

#[doc(hidden)]
#[macro_export]
macro_rules! prepare_field_for_mapping {
    ($self:ident.$field:ident) => {
        $self.$field
    };

    ($self:ident.$field:ident => $conv_expr:expr) => {{
        let $field = &$self.$field;
        $conv_expr
    }};
}

// TODO: check generated machine code and see if it is worth it to generate a switch
#[doc(hidden)]
#[macro_export]
macro_rules! unpack_variant_tag {
    ($self:ident, $current_block_tag:ident, $current_long_tag:ident, $($tag:ident)::+) => {
        $crate::unpack_variant_tag!($self, $current_block_tag, $current_long_tag, $($tag)::+ => $($tag)::+)
    };

    ($self:ident, $current_block_tag:ident, $current_long_tag:ident, $($tag:ident)::+ => $conv:expr) => {
        if $self.is_long() && $crate::internal::int_val(unsafe { $self.raw() }) == $current_long_tag {
            return Ok($conv);
        }
        $current_long_tag += 1;
    };

    ($self:ident, $current_block_tag:ident, $current_long_tag:ident,
        $($tag:ident)::+ ($($slot_name:ident: $slot_typ:ty),+)) => {

        $crate::unpack_variant_tag!(
            $self, $current_block_tag, $current_long_tag,
            $($tag)::+ ($($slot_name: $slot_typ),+) => $($tag)::+($($slot_name),+))
    };

    ($self:ident, $current_block_tag:ident, $current_long_tag:ident,
        $($tag:ident)::+ ($($slot_name:ident: $slot_typ:ty),+) => $conv:expr) => {

        if $self.is_block() && $self.tag_value() == $current_block_tag {
            let mut current_field = 0;

            $(
                let $slot_name = unsafe { $self.field::<$slot_typ>(current_field).to_rust() };
                current_field += 1;
            )+

            return Ok($conv);
        }
        $current_block_tag += 1;
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! ocaml_alloc_variant_match {
    // Base case, generate `match` expression
    ($cr:ident, $self:ident, $current_block_tag:expr, $current_long_tag:expr,

        @units {
            $({ $($unit_tag:ident)::+ @ $unit_tag_counter:expr })*
        }
        @blocks {
            $({ $($block_tag:ident)::+ ($($block_slot_name:ident: $block_slot_typ:ty),+) @ $block_tag_counter:expr })*
        }

        @pending
    ) => {
        match $self {
            $(
                $($unit_tag)::+ =>
                    $crate::OCamlAllocResult::of(unsafe { $crate::OCaml::of_i64_unchecked($unit_tag_counter as i64).raw() }),
            )*
            $(
                $($block_tag)::+($($block_slot_name),+) =>
                    $crate::ocaml_alloc_tagged_block!($cr, $block_tag_counter, $($block_slot_name: $block_slot_typ),+),
            )*
        }
    };

    // Found unit tag, add to accumulator and increment unit variant tag number
    ($cr:ident, $self:ident, $current_block_tag:expr, $current_long_tag:expr,

        @units { $($unit_tags_accum:tt)* }
        @blocks { $($block_tags_accum:tt)* }

        @pending
            { $($found_tag:ident)::+ }
            $($tail:tt)*
    ) => {
        $crate::ocaml_alloc_variant_match!{
            $cr, $self, $current_block_tag, {1u8 + $current_long_tag},

            @units {
                $($unit_tags_accum)*
                { $($found_tag)::+ @ $current_long_tag }
            }
            @blocks { $($block_tags_accum)* }

            @pending $($tail)*
        }
    };

    // Found block tag, add to accumulator and increment block variant tag number
    ($cr:ident, $self:ident, $current_block_tag:expr, $current_long_tag:expr,

        @units { $($unit_tags_accum:tt)* }
        @blocks { $($block_tags_accum:tt)* }

        @pending
            { $($found_tag:ident)::+ ($($found_slot_name:ident: $found_slot_typ:ty),+) }
            $($tail:tt)*
    ) => {
        $crate::ocaml_alloc_variant_match!{
            $cr, $self, {1u8 + $current_block_tag}, $current_long_tag,

            @units { $($unit_tags_accum)* }
            @blocks {
                $($block_tags_accum)*
                { $($found_tag)::+ ($($found_slot_name: $found_slot_typ),+) @ $current_block_tag }
            }

            @pending $($tail)*
        }
    };
}

// TODO: check generated machine code and see if it is worth it to generate a switch
#[doc(hidden)]
#[macro_export]
macro_rules! unpack_polymorphic_variant_tag {
    ($self:ident, $tag:ident => $conv:expr) => {
        #[allow(non_snake_case)]
        let $tag = $crate::polymorphic_variant_tag_hash!($tag);
        if $self.is_long() && unsafe { $self.raw() } == $tag {
            return Ok($conv);
        }
    };

    ($self:ident, $tag:ident($slot_name:ident: $slot_typ:ty) => $conv:expr) => {
        #[allow(non_snake_case)]
        let $tag = $crate::polymorphic_variant_tag_hash!($tag);

        if $self.is_block_sized(2) &&
            $self.tag_value() == $crate::internal::tag::TAG_POLYMORPHIC_VARIANT &&
            unsafe { $self.field::<$crate::OCamlInt>(0).raw() } == $tag {

            let $slot_name = unsafe { $self.field::<$slot_typ>(1).to_rust() };

            return Ok($conv);
        }
    };

    ($self:ident, $tag:ident($($slot_name:ident: $slot_typ:ty),+) => $conv:expr) => {
        #[allow(non_snake_case)]
        let $tag = $crate::polymorphic_variant_tag_hash!($tag);

        if $self.is_block_sized(2) &&
            $self.tag_value() == $crate::internal::tag::TAG_POLYMORPHIC_VARIANT &&
            unsafe { $self.field::<$crate::OCamlInt>(0).raw() } == $tag {

            let ($($slot_name),+) = unsafe { $self.field::<($($slot_typ),+)>(1).to_rust() };

            return Ok($conv);
        }
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! ocaml_closure_reference {
    ($var:ident, $name:ident) => {
        static name: &str = stringify!($name);
        static mut OC: Option<$crate::internal::OCamlClosure> = None;
        static INIT: ::std::sync::Once = ::std::sync::Once::new();
        let $var = unsafe {
            INIT.call_once(|| {
                OC = $crate::internal::OCamlClosure::named(name);
            });
            OC.unwrap_or_else(|| panic!("OCaml closure with name '{}' not registered", name))
        };
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! gcmark_result {
    ($cr:ident, $obj:expr) => {
        match $obj {
            Ok(t) => Ok(t.mark($cr).eval($cr)),
            Err(e) => Err(e),
        }
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! default_to_ocaml_unit {
    // No return value, default to unit
    () => ($crate::OCaml<()>);

    // Return value specified
    ($rtyp:ty) => ($rtyp);
}

#[doc(hidden)]
#[macro_export]
macro_rules! default_to_unit {
    // No return value, default to unit
    () => {
        ()
    };

    // Return value specified
    ($rtyp:ty) => {
        $rtyp
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! expand_rooted_args_init {
    // No more args
    ((), ) => ();

    // Nothing is done for unboxed floats
    ((), $arg:ident : f64) => ();

    (($($roots:ident)*), $arg:ident : f64, $($args:tt)*) =>
        ($crate::expand_rooted_args_init!(($($roots)*), $($args)*));

    // Other values are wrapped in `OCamlRooted<T>` as given the same lifetime as the OCaml runtime handle borrow.
    (($root:ident), $arg:ident : $typ:ty) =>
        (let $arg : $typ = unsafe { $root.keep_raw($arg) };);

    (($root:ident $($roots:ident)*), $arg:ident : $typ:ty, $($args:tt)*) => {
        let $arg : $typ = unsafe { $root.keep_raw($arg) };
        $crate::expand_rooted_args_init!(($($roots)*), $($args)*)
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! expand_exported_function {
    // Final expansions, with all argument types converted

    // If there are no roots, don't open a frame

    {
        @name $name:ident
        @cr $cr:ident
        @roots { }
        @final_args { $($arg:ident : $typ:ty,)+ }
        @proc_args { $(,)? }
        @return { $($rtyp:tt)* }
        @body $body:block
        @original_args $($original_args:tt)*
    } => {
        #[no_mangle]
        pub extern "C" fn $name( $($arg: $typ),* ) -> $crate::expand_exported_function_return!($($rtyp)*) {
            let $cr = unsafe { &mut $crate::OCamlRuntime::recover_handle() };
            $crate::expand_exported_function_body!(
                @body $body
                @return $($rtyp)*
            )
        }
    };

    // If there are roots, open a new frame and root the arguments

    {
        @name $name:ident
        @cr $cr:ident
        @roots { $($roots:ident)* }
        @final_args { $($arg:ident : $typ:ty,)+ }
        @proc_args { $(,)? }
        @return { $($rtyp:tt)* }
        @body $body:block
        @original_args $($original_args:tt)*
    } => {
        #[no_mangle]
        pub extern "C" fn $name( $($arg: $typ),* ) -> $crate::expand_exported_function_return!($($rtyp)*) {
            let $cr = unsafe { &mut $crate::OCamlRuntime::recover_handle() };
            $crate::ocaml_frame!($cr, ($($roots),*), {
                $crate::expand_rooted_args_init!(($($roots)*), $($original_args)*);
                $crate::expand_exported_function_body!(
                    @body $body
                    @return $($rtyp)*
                )
            })
        }
    };

    // Args processing

    // Next arg is an unboxed float, leave as-is

    {
        @name $name:ident
        @cr $cr:ident
        @roots { $($roots:ident)* }
        @final_args { $($final_args:tt)* }
        @proc_args { $next_arg:ident : f64, $($proc_args:tt)* }
        @return { $($rtyp:tt)* }
        @body $body:block
        @original_args $($original_args:tt)*
    } => {
        $crate::expand_exported_function!{
            @name $name
            @cr $cr
            @roots { $($roots)* }
            @final_args { $($final_args)* $next_arg : f64, }
            @proc_args { $($proc_args)* }
            @return { $($rtyp)* }
            @body $body
            @original_args $($original_args)*
        }
    };

    // Next arg is not an unboxed float, replace with RawOCaml in output, add a root

    {
        @name $name:ident
        @cr $cr:ident
        @roots { $($roots:ident)* }
        @final_args { $($final_args:tt)* }
        @proc_args { $next_arg:ident : $typ:ty, $($proc_args:tt)* }
        @return { $($rtyp:tt)* }
        @body $body:block
        @original_args $($original_args:tt)*
    } => {
        $crate::expand_exported_function!{
            @name $name
            @cr $cr
            @roots { $($roots)* root }
            @final_args { $($final_args)* $next_arg : $crate::RawOCaml, }
            @proc_args { $($proc_args)* }
            @return { $($rtyp)* }
            @body $body
            @original_args $($original_args)*
        }
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! expand_exported_function_body {
    { @body $body:block @return f64 } => {
        #[allow(unused_braces)]
        $body
    };

    { @body $body:block @return $rtyp:ty } => {{
        let retval : $rtyp = $body;
        unsafe { retval.raw() }
    }};

    { @body $body:block @return } => {
        $crate::expand_exported_function_body!(
            @body $body
            @return $crate::OCaml<()>
        )
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! expand_exported_function_return {
    () => {
        $crate::RawOCaml
    };

    (f64) => {
        f64
    };

    ($rtyp:ty) => {
        $crate::RawOCaml
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! polymorphic_variant_tag_hash {
    ($tag:ident) => {{
        static mut TAG_HASH: $crate::RawOCaml = 0;
        static INIT_TAG_HASH: std::sync::Once = std::sync::Once::new();
        unsafe {
            INIT_TAG_HASH.call_once(|| {
                TAG_HASH =
                    $crate::internal::caml_hash_variant(concat!(stringify!($tag), "\0").as_ptr())
            });
            TAG_HASH
        }
    }};
}
