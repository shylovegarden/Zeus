import React from 'react';
import { BrowserRouter as Router, Routes, Route } from 'react-router-dom';
import { QueryClient, QueryClientProvider } from 'react-query';
import { Toaster } from 'react-hot-toast';

import Navbar from './components/Navbar';
import Dashboard from './pages/Dashboard';
import Compile from './pages/Compile';
import Certificates from './pages/Certificates';
import Libraries from './pages/Libraries';
import './index.css';

const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      retry: 1,
      staleTime: 30000,
    },
  },
});

function App() {
  return (
    <QueryClientProvider client={queryClient}>
      <Router>
        <div className="min-h-screen bg-gray-50">
          <Navbar />
          <main className="container mx-auto px-4 py-8">
            <Routes>
              <Route path="/" element={<Dashboard />} />
              <Route path="/compile" element={<Compile />} />
              <Route path="/certificates" element={<Certificates />} />
              <Route path="/libraries" element={<Libraries />} />
            </Routes>
          </main>
          <Toaster position="bottom-right" />
        </div>
      </Router>
    </QueryClientProvider>
  );
}

export default App;
