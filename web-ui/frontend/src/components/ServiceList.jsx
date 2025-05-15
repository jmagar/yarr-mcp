import React, { useEffect, useState } from 'react';

function ServiceList() {
  const [services, setServices] = useState([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState(null);

  useEffect(() => {
    async function fetchServices() {
      try {
        // Assuming the backend is running on port 8081
        const response = await fetch('http://localhost:8081/api/mcp-services');
        if (!response.ok) {
          throw new Error(`HTTP error! status: ${response.status}`);
        }
        const data = await response.json();
        setServices(data);
      } catch (e) {
        setError(e.message);
      } finally {
        setLoading(false);
      }
    }

    fetchServices();
  }, []);

  if (loading) return <p>Loading services...</p>;
  if (error) return <p>Error loading services: {error}</p>;

  return (
    <div className="container mx-auto p-4">
      <h1 className="text-2xl font-bold mb-4">MCP Services Status</h1>
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
        {/* {services.map(service => (
          <ServiceCard key={service.name} service={service} />
        ))} */}
        {services.length > 0 ? (
          services.map(service => (
            <div key={service.name} className="p-4 border rounded shadow">
              <h2 className="text-xl font-semibold">{service.name}</h2>
              <p>Enabled: {service.enabled ? 'Yes' : 'No'}</p>
              <p>URL: {service.mcp_url || 'N/A'}</p>
              {/* Placeholder for health status */}
            </div>
          ))
        ) : (
          <p>No services configured or found.</p>
        )}
      </div>
    </div>
  );
}

export default ServiceList; 