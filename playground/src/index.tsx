import {createRoot} from "react-dom/client";
import {BrowserRouter, Link, Route, Routes} from "react-router";
import App from "@/App.tsx";

const elem = document.getElementById("root")!;
const app = (
    <BrowserRouter>
        <nav style={{display: "flex", gap: "1rem", marginBottom: "1rem"}}>
            <Link to="/">Home</Link>
            <Link to="/about">About</Link>
            <Link to="/contact">Contact</Link>
        </nav>

        <Routes>
            <Route path="/" element={<App/>}/>
            <Route path="/about" element={<h2>ℹ️ About Page</h2>}/>
            <Route path="/contact" element={<h2>📞 Contact Page</h2>}/>
        </Routes>
    </BrowserRouter>
);

createRoot(elem).render(app);
