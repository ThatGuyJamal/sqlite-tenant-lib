SQLite Multi-Tenant Management Library

---

I have always enjoyed using [sqlite](https://www.sqlite.org/index.html) in my projects. I've created this project to make it easier for 
me and others to manage multiple databases in the same application. 

For example if you have an app, and you want user data to be seperated into different databases, this
can be tricky to set up and time-consuming to make sure it works properly. So ive written some utility to 
make that process easier. Its 100% in pure rust and if this project gains traction I will consider porting 
bindings over to other languages (py, js, ...). Make a PR if you want to help.

What im working on currently can be found in the [todo.md](./todo.md) file.

[Mit Licence](./License)

---

- [Sqlite docs](https://sqlite.org)

- The library core is built on [rust-sqlite](https://docs.rs/rusqlite/0.31.0/rusqlite/index.html). You can read
the docs here to understand how to access and use sqlite from this crate.
- Example usage of the library can be found in [./examples](./examples)

---

If you need database migration support, I recommend using the [refinery](https://github.com/rust-db/refinery) library. After
some thinking, I did not want to attempt to implement migrations myself as its complicated and smarter people have done it already.