pub const PRELUDE: &'static str = r#"
(def! not (fn* (a) (if a false true)))
(def! load-file (fn* (f) (eval (read-string (str "(do " (slurp f) "\nnil)")))))
"#;
