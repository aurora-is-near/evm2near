;; converts all int/long literals to hex
;; run with Babashka (https://github.com/babashka/babashka#installation)
;; bb int-to-hex.clj <input-file> <output-file>

(require '[clojure.string :as str]
         '[clojure.java.io])

(defn convert-to-hex [[whole spacing number & other]]
  (let [num (Long/parseLong number)
        hex-num (if (int? num) (Integer/toHexString num)
                               (Long/toHexString num))]
    (str spacing "0x" hex-num)))

(let [[input-file output-file] *command-line-args*]
  (->> (slurp input-file)
       (str/split-lines)
       (map (fn [line] (str/replace line #"(\s)(-?\d+)\b" convert-to-hex)))
       (str/join "\n")
       (spit output-file)))
