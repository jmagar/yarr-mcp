import httpx
import os
import logging
from typing import Optional, Dict, Any, Union, List

log = logging.getLogger(__name__)

class TautulliApiClient:
    def __init__(self, base_url: Optional[str], api_key: Optional[str]):
        if not base_url:
            log.error("Tautulli API client initialized without a base URL.")
            raise ValueError("Tautulli base URL is required.")
        if not api_key:
            log.error("Tautulli API client initialized without an API key.")
            raise ValueError("Tautulli API key is required.")
            
        self.base_url = base_url.rstrip('/') + "/api/v2" # Tautulli API is typically at /api/v2
        self.api_key = api_key
        self.default_params = {"apikey": self.api_key}
        self._client = httpx.AsyncClient()
        log.info(f"TautulliApiClient initialized for URL: {self.base_url}")

    async def close(self):
        await self._client.aclose()
        log.info("TautulliApiClient closed.")

    async def _request(self, cmd: str, params: Optional[Dict[str, Any]] = None) -> Union[Dict, str]:
        request_params = self.default_params.copy()
        request_params["cmd"] = cmd
        if params:
            # Ensure all additional params are suitable for URL query (e.g., stringify)
            for key, value in params.items():
                if isinstance(value, bool):
                    request_params[key] = '1' if value else '0' 
                elif value is not None: 
                    request_params[key] = str(value)
        
        url = self.base_url # The full path including /api/v2 is already in self.base_url
        log.debug(f"Tautulli API Request: GET {url} | Params: {request_params}")

        try:
            response = await self._client.get(url, params=request_params)
            response.raise_for_status()
            json_response = response.json()
            log.debug(f"Tautulli API Response: {json_response}")

            # Tautulli API nests actual data under response.data
            if isinstance(json_response, dict) and json_response.get("response", {}).get("result") == "success":
                return json_response.get("response", {}).get("data", {})
            else:
                error_message = json_response.get("response", {}).get("message", "Unknown Tautulli API error")
                log.error(f"Tautulli API returned error: {error_message} | Full response: {json_response}")
                return f"Error: Tautulli API - {error_message}"

        except httpx.HTTPStatusError as e:
            log.error(f"Tautulli API HTTP Error: {e.response.status_code} for {e.request.url} - Response: {e.response.text}")
            return f"Error: Tautulli API request failed ({e.response.status_code})."
        except httpx.RequestError as e:
            log.error(f"Tautulli API Request Error: {e} for {e.request.url}")
            return f"Error: Failed to connect to Tautulli. Details: {e}"
        except ValueError: # JSONDecodeError
            log.error(f"Tautulli API JSON Decode Error for {url}. Response: {response.text}")
            return "Error: Failed to decode Tautulli API response as JSON."
        except Exception as e:
            log.error(f"Unexpected error during Tautulli API request to {url}: {e}", exc_info=True)
            return f"Error: An unexpected error occurred. Details: {e}"

    # --- Specific command methods ---
    async def get_activity(self) -> Union[Dict, str]:
        return await self._request("get_activity")

    async def get_home_stats(self) -> Union[Dict, str]:
        return await self._request("get_home_stats")

    async def get_history(self, user_id: Optional[int]=None, section_id: Optional[str]=None, length: int=25) -> Union[Dict, str]:
        params = {"length": length}
        if user_id is not None: params["user_id"] = user_id
        # Tautulli API uses section_id for library filtering in get_history, not library_name
        if section_id is not None: params["section_id"] = section_id 
        return await self._request("get_history", params)

    async def get_libraries(self) -> Union[List[Dict], str]: # Expects a list of libraries
        return await self._request("get_libraries")

    async def get_users(self) -> Union[List[Dict], str]: # Expects a list of users
        return await self._request("get_users") 