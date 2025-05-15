import { useState, useEffect } from 'react'
import './App.css'
import ServiceCard from './components/ServiceCard'
import LogViewer from './components/LogViewer'
import { Button } from './components/ui/button'
import { Terminal, RefreshCw } from 'lucide-react'

// Define the structure of the service object we expect from the backend
interface ServiceConfig {
  name: string
  mcp_url: string | null
  mcp_host_inferred: string | null
  mcp_port: string | null
  enabled: boolean
}

function App() {
  const [services, setServices] = useState<ServiceConfig[]>([])
  const [loading, setLoading] = useState<boolean>(true)
  const [error, setError] = useState<string | null>(null)

  // State for Log Viewer
  const [isLogViewerOpen, setIsLogViewerOpen] = useState(false)
  const [currentLogs, setCurrentLogs] = useState('')
  const [logContainerName, setLogContainerName] = useState('')
  const [logError, setLogError] = useState<string | null>(null)
  const [isFetchingLogs, setIsFetchingLogs] = useState(false)

  useEffect(() => {
    const fetchServices = async () => {
      try {
        // Corrected API endpoint based on backend structure
        const response = await fetch('http://127.0.0.1:8081/api/mcp-services')
        if (!response.ok) {
          throw new Error(`Failed to fetch services: ${response.status} ${response.statusText}`)
        }
        const data: ServiceConfig[] = await response.json()
        console.log("[App.tsx] Fetched services data from backend:", data)
        setServices(data)
      } catch (err) {
        if (err instanceof Error) {
          setError(err.message)
        } else {
          setError('An unknown error occurred')
        }
        console.error("Error fetching services:", err)
      } finally {
        setLoading(false)
      }
    }

    fetchServices()
  }, [])

  // Function to fetch and display yarr-mcp logs
  const handleFetchYarrMcpLogs = async (tailLines: number = 100) => {
    setIsFetchingLogs(true)
    setLogError(null)
    try {
      const response = await fetch(`http://127.0.0.1:8081/api/logs/yarr-mcp?tail=${tailLines}`)
      if (!response.ok) {
        let errText = response.statusText
        try {
          const errData = await response.json()
          errText = errData.error || errData.message || response.statusText
        } catch { /* ignore json parsing error if response not json */ }
        throw new Error(`Failed to fetch logs: ${response.status} ${errText}`)
      }
      const data = await response.json()
      if (data.error) {
        throw new Error(data.error)
      }
      setCurrentLogs(data.logs)
      setLogContainerName(data.container_name || 'yarr-mcp')
      setIsLogViewerOpen(true)
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : 'An unknown error occurred while fetching logs.'
      console.error("Error fetching yarr-mcp logs:", errorMessage)
      setLogError(errorMessage)
      // Optionally open the dialog to show the error, or handle differently
      setCurrentLogs(`Error fetching logs: ${errorMessage}`)
      setLogContainerName('yarr-mcp')
      setIsLogViewerOpen(true)
    } finally {
      setIsFetchingLogs(false)
    }
  }

  if (loading) {
    return (
      <div className="flex justify-center items-center min-h-screen bg-background text-foreground">
        <p className="text-xl">Loading services...</p>
      </div>
    )
  }

  if (error) {
    return (
      <div className="flex flex-col justify-center items-center min-h-screen bg-background text-destructive-foreground p-4">
        <h1 className="text-2xl font-bold mb-4">Error</h1>
        <p className="text-lg bg-destructive p-4 rounded-md">{error}</p>
        <p className="mt-4 text-sm">Please ensure the backend server is running and accessible.</p>
      </div>
    )
  }

  return (
    <div className="min-h-screen bg-background text-foreground p-4 sm:p-6 md:p-8">
      <header className="mb-8 text-center">
        <div className="flex justify-between items-center mb-4">
          <div>{/* Placeholder for left content if any */}</div>
          <h1 className="text-4xl font-bold text-primary">YARR MCP Dashboard</h1>
          <Button 
            variant="outline" 
            size="icon" 
            onClick={() => handleFetchYarrMcpLogs(200)} 
            disabled={isFetchingLogs}
            title="View yarr-mcp logs"
          >
            {isFetchingLogs ? <RefreshCw className="h-5 w-5 animate-spin" /> : <Terminal className="h-5 w-5" />}
          </Button>
        </div>
        <p className="text-muted-foreground">Oversee and manage your MCP services.</p>
      </header>
      
      {services.length === 0 ? (
        <div className="text-center">
          <p className="text-xl">No services found.</p>
          <p className="text-muted-foreground">Make sure your .env file is configured correctly and the backend can read it.</p>
        </div>
      ) : (
        <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-6">
          {services.map((service) => (
            <ServiceCard key={service.name} service={service} />
          ))}
        </div>
      )}
      
      <footer className="mt-12 text-center text-sm text-muted-foreground">
        <p>&copy; {new Date().getFullYear()} YARR MCP. All rights reserved.</p>
      </footer>

      <LogViewer 
        isOpen={isLogViewerOpen} 
        onOpenChange={setIsLogViewerOpen} 
        logs={currentLogs} 
        containerName={logContainerName} 
      />
      {logError && (
        <div className="fixed bottom-4 right-4 bg-destructive text-destructive-foreground p-3 rounded-md shadow-lg">
          <p>Log Error: {logError}</p>
          <Button variant="ghost" size="sm" onClick={() => setLogError(null)} className="ml-2">Dismiss</Button>
        </div>
      )}
    </div>
  )
}

export default App
