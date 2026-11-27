(module
  (import "mundam:probe/preview@0.1.0" "read"
    (func $read (param i32 i32 i32) (result i32)))
  (memory (export "memory") 1)
  (func (export "run") (result i32)
    i32.const 0
    i32.const 4
    i32.const 16
    call $read
    drop
    i32.const 4
    i32.const 4
    i32.const 0
    call $read
    drop
    i32.const 16
    i32.load))
