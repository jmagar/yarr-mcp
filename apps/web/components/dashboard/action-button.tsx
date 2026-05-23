import { Button } from "@/components/ui/button";

export function ActionButton({ onClick, label }: { onClick: () => void; label: string }) {
  return (
    <Button type="button" onClick={onClick} variant="aurora">
      {label}
    </Button>
  );
}
