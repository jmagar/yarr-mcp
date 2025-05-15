import React from 'react';
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogDescription, DialogFooter } from './ui/dialog';
import { Button } from './ui/button';
import { ScrollArea } from './ui/scroll-area'; // For scrollable log content

interface LogViewerProps {
  logs: string;
  containerName: string;
  isOpen: boolean;
  onOpenChange: (isOpen: boolean) => void;
}

const LogViewer: React.FC<LogViewerProps> = ({ logs, containerName, isOpen, onOpenChange }) => {
  return (
    <Dialog open={isOpen} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-4xl max-h-[80vh] flex flex-col">
        <DialogHeader>
          <DialogTitle>Logs: {containerName}</DialogTitle>
          <DialogDescription>
            Showing recent logs from the container.
          </DialogDescription>
        </DialogHeader>
        <ScrollArea className="rounded-md border p-4 bg-secondary/50 h-[60vh]">
          <pre className="text-xs whitespace-pre-wrap break-all">
            {logs ? logs : 'No logs to display or logs are empty.'}
          </pre>
        </ScrollArea>
        <DialogFooter className="sm:justify-start">
          <Button type="button" variant="secondary" onClick={() => onOpenChange(false)}>
            Close
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
};

export default LogViewer; 