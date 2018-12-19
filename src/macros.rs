#[macro_export]
macro_rules! ruby_class {
    ($outer_class:ident$(::$inner_class:ident)*) => (
        rutie::Class::from_existing(stringify!($outer_class)) $(
            .get_nested_class(stringify!($inner_class))
        )*
    )
}

/// A macro to define Rutie methods, which uses `rutie_serde` to deserialize arguments and serialize results.
///
/// In comparison to `rutie::methods!`, this macro:
///
///  - Attempts to use `rutie_serde` to deserialize into the required type for each argument.
///  - Allows methods to return either `Result<T, E>` or `T`, where `T: IntoAnyObject` and
///    `E: IntoException`. Errors are safely raised as Ruby exceptions and successful computations
///    are serialized into Ruby objects using `rutie_serde`.
///  - Catches any panics that occur during the execution of each method's body, and re-raises
///    them as a Ruby exception.
///  - Catches any errors that occur during `rutie_serde` deserialization/serialization and safely
///    raises them as Ruby exceptions.
///
/// It accepts an extra `exception_class` argument, which should be an expression resulting in a
/// `rutie::Class` which is used to instantiate exceptions that are raised from panics.
#[macro_export]
macro_rules! rutie_serde_methods {
    // This macro is recursive and defines one method each time it recurses. This is the base-case
    // where there are no more methods to define.
    (
        $itself_class:ty,
        $itself_name:ident,
        $exception_class:expr,
    ) => {};

    // Define a method that returns a `Result<T, E>` where `T: IntoAnyObject, E: IntoException`.
    (
        $itself_class:ty,
        $itself_name:ident,
        $exception_class:expr,

        fn $method_name:ident($($arg_name:ident: $arg_type:ty),* $(,)*) -> Result<$return_type:ty, $error_type:ty>
        $body:block

        $($other_methods:tt)*
    ) => {
        #[allow(unused_imports)]
        pub extern fn $method_name(argc: ::rutie::types::Argc,
                                    argv: *const ::rutie::AnyObject,
                                    mut $itself_name: $itself_class) -> ::rutie::AnyObject {
            // Be careful with heap allocations at this top-level - try to place them inside
            // the closure. raise_ruby_exception() will call rb_raise() (longjmp) without
            // letting Rust cleanup first.
            use ::std::result::Result;
            use rutie;
            use $crate::{self, DeserializeWrapper, IntoAnyObject, IntoException, ResultExt};
            use $crate::panics::catch_and_raise;

            enum ClosureError {
                RutieSerde($crate::Error),
                Body($error_type),
            }

            impl IntoException for ClosureError {
                fn into_exception(self, default_class: rutie::Class) -> rutie::AnyException {
                    match self {
                        ClosureError::RutieSerde(error) => IntoException::into_exception(error, default_class),
                        ClosureError::Body(error) => IntoException::into_exception(error, default_class),
                    }
                }
            }

            impl From<$crate::Error> for ClosureError {
                fn from(error: $crate::Error) -> ClosureError {
                    ClosureError::RutieSerde(error)
                }
            }

            let result = catch_and_raise($exception_class, move || -> Result<rutie::AnyObject, ClosureError> {
                let _arguments = rutie::util::parse_arguments(argc, argv);
                #[allow(unused_mut)]
                let mut _i = 0;

                $(
                    let $arg_name: $arg_type =
                        _arguments
                            .get(_i)
                            .ok_or_else(|| {
                                let err: rutie_serde::Error =
                                    format!(
                                        "Argument '{}: {}' not found for method '{}'",
                                        stringify!($arg_name),
                                        stringify!($arg_type),
                                        stringify!($method_name)
                                    ).into();
                                err
                            })
                            .map_err($crate::Error::from)
                            .and_then(|object| DeserializeWrapper::deserialize(object))
                            .chain_context(|| format!("When deserializing arg: {}", stringify!($arg_name)))
                            .map_err(ClosureError::RutieSerde)?;

                    _i += 1;
                )*

                #[allow(unused_variables)]
                let result: Result<$return_type, _> = $body;

                #[allow(unreachable_code)]
                result
                    .map_err(ClosureError::Body)
                    .and_then(|return_value| {
                        IntoAnyObject::into_any_object(return_value)
                            .map_err(ClosureError::RutieSerde)
                    })
            });

            match result {
                Ok(value) => value,
                Err(error) => {
                    let exception = error.into_exception($exception_class);
                    rutie::VM::raise_ex(exception);
                    unreachable!("::rutie::VM::raise_ex")
                }
            }
        }

        // Recurse and define the rest of the methods.
        rutie_serde_methods!(
            $itself_class,
            $itself_name,
            $exception_class,

            $($other_methods)*
        );
    };

    // Define a method that returns a `T` (i.e. not `Result`) by wrapping its return value in
    // a `Result` and recursing again. We use an error type of `rutie_serde::Error`, but it
    // is never actually used.
    (
        $itself_class:ty,
        $itself_name:ident,
        $exception_class:expr,

        fn $method_name:ident($($arg_name:ident: $arg_type:ty),* $(,)*) -> $return_type:ty
        $body:block

        $($other_methods:tt)*
    ) => {
        rutie_serde_methods!(
            $itself_class,
            $itself_name,
            $exception_class,

            fn $method_name($($arg_name:$arg_type),*)
                -> Result<$return_type, $crate::Error>
            {
                let return_value = $body;

                #[allow(unreachable_code)]
                Ok(return_value)
            }

            $($other_methods)*
        );
    };
}
