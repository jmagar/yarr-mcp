import React, { useState, useEffect, useCallback } from 'react';
import { Card, CardContent, CardDescription, CardFooter, CardHeader, CardTitle } from './ui/card';
import { Badge } from './ui/badge';
import { Button } from './ui/button';
import { CircleAlert, CircleCheck, CircleDashed, RefreshCw } from 'lucide-react';

// Define the structure of the service object we expect
interface ServiceConfig {
  name: string;
  mcp_url: string | null;
  mcp_host_inferred: string | null;
  mcp_port: string | null;
  enabled: boolean;
}

// Define the structure of the health check response
interface HealthStatus {
  status: 'ok' | 'error' | 'checking' | 'disabled' | 'unavailable';
  service_accessible?: boolean;
  reason?: string;
  details?: Record<string, unknown> | string; 
}

interface ServiceCardProps {
  service: ServiceConfig;
}

const ServiceCard: React.FC<ServiceCardProps> = (props) => {
  console.log(`ServiceCard: TOP LEVEL LOG FOR ${props.service.name}`);
  const { service } = props;
  console.log(`[${service.name}] ServiceCard rendering. Enabled: ${service.enabled}, Port: ${service.mcp_port}`);

  const [health, setHealth] = useState<HealthStatus>({
    status: service.enabled ? 'checking' : 'disabled',
  });
  const [isLoading, setIsLoading] = useState(false);

  const getHealthCheckUrl = useCallback((): string | null => {
    if (!service.mcp_port) return null;
    return `http://localhost:${service.mcp_port}/health`;
  }, [service.mcp_port]);

  const checkHealth = useCallback(async () => {
    if (!service.enabled) {
      setHealth({ status: 'disabled' });
      return;
    }

    const healthUrl = getHealthCheckUrl();
    if (!healthUrl) {
      setHealth({ status: 'unavailable', reason: 'MCP URL not configured for service.' });
      return;
    }

    setIsLoading(true);
    setHealth(prev => ({ ...prev, status: 'checking' }));
    console.log(`[${service.name}] Checking health at: ${healthUrl}`);

    try {
      const response = await fetch(healthUrl);
      console.log(`[${service.name}] Health check response status: ${response.status}`);
      
      if (!response.ok) {
        let errorReason = response.statusText;
        try {
          const errorData = await response.json();
          console.log(`[${service.name}] Health check error data (JSON):`, errorData);
          errorReason = errorData.reason || errorData.detail || errorData.message || response.statusText;
        } catch {
          console.log(`[${service.name}] Health check response text (non-JSON): ${await response.text().catch(() => 'Failed to read text')}`);
          // Parsing JSON failed, stick with statusText or use raw text if available
        }
        throw new Error(`HTTP error ${response.status}: ${errorReason}`);
      }
      
      const data = await response.json();
      console.log(`[${service.name}] Health check data received:`, data);
      
      setHealth({
        status: data.status === 'ok' ? 'ok' : 'error',
        service_accessible: data.service_accessible,
        reason: data.reason,
        details: data.details,
      });
    } catch (error: unknown) {
      let errorMessage = 'Failed to fetch health status.';
      if (error instanceof Error) {
        errorMessage = error.message;
      }
      console.error(`Error checking health for ${service.name}:`, error);
      setHealth({
        status: 'error',
        reason: errorMessage,
      });
    } finally {
      setIsLoading(false);
    }
  }, [service.enabled, service.name, getHealthCheckUrl]);

  useEffect(() => {
    console.log(`[${service.name}] useEffect for checkHealth triggered.`);
    checkHealth();
  }, [checkHealth, service.name]);

  const getStatusIcon = () => {
    switch (health.status) {
      case 'ok':
        return <CircleCheck className="h-5 w-5 text-green-700 dark:text-green-400" />;
      case 'error':
        return <CircleAlert className="h-5 w-5 text-red-700 dark:text-red-400" />;
      case 'checking':
        return <RefreshCw className="h-5 w-5 text-yellow-700 dark:text-yellow-400 animate-spin" />;
      case 'disabled':
      case 'unavailable':
        return <CircleDashed className="h-5 w-5 text-gray-600 dark:text-gray-400" />;
      default:
        return <CircleDashed className="h-5 w-5 text-gray-600 dark:text-gray-400" />;
    }
  };

  return (
    <Card className="w-full max-w-sm">
      <CardHeader>
        <div className="flex items-center justify-between">
          <CardTitle className="text-lg">{service.name}</CardTitle>
          {getStatusIcon()}
        </div>
        <CardDescription>
          Status: {health.status.charAt(0).toUpperCase() + health.status.slice(1)}
          {!service.enabled && <Badge variant="outline" className="ml-2 bg-gray-200 text-gray-700">Disabled</Badge>}
          {service.enabled && health.status === 'ok' && <Badge variant="outline" className="ml-2 bg-green-200 text-green-800">Enabled & Healthy</Badge>}
          {service.enabled && health.status === 'error' && <Badge variant="destructive" className="ml-2">Enabled & Error</Badge>}
           {service.enabled && health.status === 'checking' && <Badge variant="outline" className="ml-2 bg-yellow-200 text-yellow-800">Checking...</Badge>}
        </CardDescription>
      </CardHeader>
      <CardContent className="text-sm space-y-1">
        <p><strong>MCP URL:</strong> {service.mcp_url || 'N/A'}</p>
        <p><strong>MCP Host:</strong> {service.mcp_host_inferred || 'N/A'}</p>
        <p><strong>MCP Port:</strong> {service.mcp_port || 'N/A'}</p>
        {health.status === 'ok' && health.service_accessible && (
          <p className="text-green-600 dark:text-green-400">Target application is accessible.</p>
        )}
        {health.status === 'ok' && health.service_accessible === false && (
           <p className="text-orange-600 dark:text-orange-400">Target application is NOT accessible. Reason: {health.reason || 'Unknown'}</p>
        )}
        {health.status === 'error' && (
          <p className="text-red-600 dark:text-red-400">Error: {health.reason || 'Unknown error'}</p>
        )}
        {health.status === 'unavailable' && (
          <p className="text-orange-600 dark:text-orange-400">MCP Service Unavailable: {health.reason || 'URL not configured or invalid.'}</p>
        )}
      </CardContent>
      <CardFooter>
        {service.enabled && (
          <Button onClick={checkHealth} disabled={isLoading} size="sm" variant="outline">
            {isLoading ? (
              <RefreshCw className="mr-2 h-4 w-4 animate-spin" />
            ) : (
              <RefreshCw className="mr-2 h-4 w-4" />
            )}
            Refresh Status
          </Button>
        )}
      </CardFooter>
    </Card>
  );
};

export default ServiceCard; 