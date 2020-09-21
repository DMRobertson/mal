pub const PRELUDE: &str = r#"
(def! not (fn* (a) (if a false true)))
(def! load-file (fn* (f) (do (map eval (rest (read-string (str "(do " (slurp f) "\nnil)")))) nil)))
(defmacro! cond (fn* (& xs) (if (> (count xs) 0) (list 'if (first xs) (if (> (count xs) 1) (nth xs 1) (throw "odd number of forms to cond")) (cons 'cond (rest (rest xs)))))))
"#;
