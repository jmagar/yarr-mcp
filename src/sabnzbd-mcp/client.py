import httpx
import os
import logging
from typing import Optional, Dict, Any, Union
from urllib.parse import quote_plus, urlencode

log = logging.getLogger(__name__)

class SabnzbdApiClient:
    def __init__(self, base_url: Optional[str], api_key: Optional[str]):
        if not base_url:
            log.error("SABnzbd API client initialized without a base URL.")
            raise ValueError("SABnzbd base URL is required.")
        if not api_key:
            log.error("SABnzbd API client initialized without an API key.")
            raise ValueError("SABnzbd API key is required.")
            
        self.base_url = base_url.rstrip('/')
        self.api_key = api_key
        # Default params for all requests to SABnzbd API
        self.default_params = {"apikey": self.api_key, "output": "json"}
        self._client = httpx.AsyncClient()
        log.info(f"SabnzbdApiClient initialized for URL: {self.base_url}")

    async def close(self):
        await self._client.aclose()
        log.info("SabnzbdApiClient closed.")

    async def _request(self, mode: str, params: Optional[Dict[str, Any]] = None) -> Union[Dict, str]:
        # SABnzbd API uses query parameters for everything, including the mode.
        # The base endpoint is /sabnzbd/api
        request_params = self.default_params.copy()
        request_params["mode"] = mode
        if params:
            # Ensure all additional params are suitable for URL query (e.g., stringify)
            for key, value in params.items():
                if isinstance(value, bool):
                    request_params[key] = '1' if value else '0' # SABnzbd often uses 1/0 for bools
                elif value is not None: # Skip None values, SABnzbd treats missing params as default
                    request_params[key] = str(value)
        
        url = f"{self.base_url}/sabnzbd/api"
        # httpx will handle URL encoding of the parameters dictionary
        log.debug(f"SABnzbd API Request: GET {url} | Params: {request_params}")

        try:
            response = await self._client.get(url, params=request_params)
            response.raise_for_status() # Raises HTTPStatusError for 4xx/5xx responses
            
            json_response = response.json()
            log.debug(f"SABnzbd API Response: {json_response}")

            # SABnzbd specific error checking if it doesn't use HTTP status codes for all errors
            if isinstance(json_response, dict) and json_response.get("status") is False:
                error_message = json_response.get("error", "Unknown SABnzbd API error")
                log.error(f"SABnzbd API returned error: {error_message}")
                return f"Error: SABnzbd API - {error_message}"
            if isinstance(json_response, dict) and "error" in json_response and "status" not in json_response:
                 # Direct error message like { "error": "API Key Incorrect" }
                error_message = json_response.get("error")
                log.error(f"SABnzbd API direct error: {error_message}")
                return f"Error: SABnzbd API - {error_message}"

            return json_response
        except httpx.HTTPStatusError as e:
            log.error(f"SABnzbd API HTTP Error: {e.response.status_code} for {e.request.url} - Response: {e.response.text}")
            return f"Error: SABnzbd API request failed ({e.response.status_code})."
        except httpx.RequestError as e:
            log.error(f"SABnzbd API Request Error: {e} for {e.request.url}")
            return f"Error: Failed to connect to SABnzbd. Details: {e}"
        except ValueError: # JSONDecodeError
            log.error(f"SABnzbd API JSON Decode Error for {url}. Response: {response.text}")
            return "Error: Failed to decode SABnzbd API response as JSON."
        except Exception as e:
            log.error(f"Unexpected error during SABnzbd API request to {url}: {e}", exc_info=True)
            return f"Error: An unexpected error occurred. Details: {e}"

    async def get_queue(self, start: int = 0, limit: int = 20, category: Optional[str] = None) -> Union[Dict, str]:
        params = {"start": start, "limit": limit}
        if category: params["cat"] = category
        return await self._request("queue", params)

    async def get_history(self, start: int = 0, limit: int = 20) -> Union[Dict, str]:
        params = {"start": start, "limit": limit}
        return await self._request("history", params)

    async def pause_queue(self) -> Union[Dict, str]:
        return await self._request("pause")

    async def resume_queue(self) -> Union[Dict, str]:
        return await self._request("resume")

    async def add_nzb_by_url(self, nzb_url: str, category: Optional[str] = None) -> Union[Dict, str]:
        params = {"name": nzb_url} # 'name' is the parameter for URL for mode=addurl
        if category: params["cat"] = category
        return await self._request("addurl", params)
        
    async def set_speedlimit(self, percentage_str: str) -> Union[Dict, str]:
        # SABnzbd API for mode=speedlimit expects a string like "50" for 50%
        # or a KB/s value like "10240" for 10MB/s.
        # The PRD says `set_sab_speedlimit(percentage: int)`, so we convert int to string for the API.
        params = {"value": str(percentage_str)} 
        return await self._request("speedlimit", params) 