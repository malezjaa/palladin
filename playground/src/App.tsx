import "./index.css";
import {useState} from "react";

export function App() {
    const [count, setCount] = useState(0);

    return (
        <div style={{textAlign: "center"}}>
            <p>Current count: {count}</p>
            <button onClick={() => setCount(count + 1)}>Increment</button>
        </div>
    );
}

export default App;
