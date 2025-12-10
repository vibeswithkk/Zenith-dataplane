"""
Zenith Scheduler Client

A Python client for the Zenith job scheduler REST API.
Provides job submission, monitoring, and cluster status.
"""

import os
import time
import functools
from dataclasses import dataclass, field
from typing import Optional, List, Dict, Callable
from enum import Enum

try:
    import requests
    REQUESTS_AVAILABLE = True
except ImportError:
    REQUESTS_AVAILABLE = False


class JobState(Enum):
    """Job execution states."""
    QUEUED = "Queued"
    SCHEDULED = "Scheduled"
    RUNNING = "Running"
    COMPLETED = "Completed"
    FAILED = "Failed"
    CANCELLED = "Cancelled"
    TIMEOUT = "Timeout"


@dataclass
class JobConfig:
    """Job configuration."""
    name: str
    command: str
    arguments: List[str] = field(default_factory=list)
    environment: Dict[str, str] = field(default_factory=dict)
    working_directory: str = "/app"
    gpu_count: int = 0
    cpu_cores: int = 1
    memory_mb: int = 4096
    priority: int = 50
    gang_schedule: bool = False
    user_id: str = "default"
    project_id: str = "default"


@dataclass
class Job:
    """Represents a submitted job."""
    job_id: str
    name: str
    state: JobState
    user_id: str
    project_id: str
    created_at: str
    allocated_nodes: List[str]
    gpu_count: int
    
    @classmethod
    def from_response(cls, data: dict) -> 'Job':
        """Create Job from API response."""
        state_str = data.get('state', 'Queued')
        try:
            state = JobState(state_str)
        except ValueError:
            state = JobState.QUEUED
            
        return cls(
            job_id=data.get('job_id', ''),
            name=data.get('name', ''),
            state=state,
            user_id=data.get('user_id', ''),
            project_id=data.get('project_id', ''),
            created_at=data.get('created_at', ''),
            allocated_nodes=data.get('allocated_nodes', []),
            gpu_count=data.get('gpu_count', 0),
        )


@dataclass
class ClusterStatus:
    """Cluster status information."""
    total_nodes: int
    healthy_nodes: int
    total_gpus: int
    available_gpus: int
    running_jobs: int
    queued_jobs: int
    
    @classmethod
    def from_response(cls, data: dict) -> 'ClusterStatus':
        """Create ClusterStatus from API response."""
        return cls(
            total_nodes=data.get('total_nodes', 0),
            healthy_nodes=data.get('healthy_nodes', 0),
            total_gpus=data.get('total_gpus', 0),
            available_gpus=data.get('available_gpus', 0),
            running_jobs=data.get('running_jobs', 0),
            queued_jobs=data.get('queued_jobs', 0),
        )


@dataclass
class Node:
    """Cluster node information."""
    id: str
    hostname: str
    health: str
    total_gpus: int
    available_gpus: int
    running_jobs: int
    
    @classmethod
    def from_response(cls, data: dict) -> 'Node':
        """Create Node from API response."""
        return cls(
            id=data.get('id', ''),
            hostname=data.get('hostname', ''),
            health=data.get('health', 'Unknown'),
            total_gpus=data.get('total_gpus', 0),
            available_gpus=data.get('available_gpus', 0),
            running_jobs=data.get('running_jobs', 0),
        )


class SchedulerError(Exception):
    """Scheduler-related errors."""
    pass


class SchedulerClient:
    """
    Client for the Zenith job scheduler.
    
    Connects to the scheduler's REST API for job management.
    
    Example:
        >>> client = SchedulerClient("http://localhost:8080")
        >>> job = client.submit(JobConfig(name="train", command="python train.py", gpu_count=4))
        >>> print(f"Submitted: {job.job_id}")
        >>> 
        >>> status = client.wait(job.job_id, timeout=3600)
        >>> print(f"Final state: {status.state}")
    """
    
    def __init__(
        self, 
        base_url: Optional[str] = None,
        timeout: float = 30.0,
        retry_count: int = 3,
    ):
        """
        Initialize scheduler client.
        
        Args:
            base_url: Scheduler API URL (default: ZENITH_SCHEDULER_URL env or localhost:8080)
            timeout: Request timeout in seconds
            retry_count: Number of retries for failed requests
        """
        if not REQUESTS_AVAILABLE:
            raise ImportError(
                "requests package required for scheduler client. "
                "Install with: pip install requests"
            )
            
        self.base_url = base_url or os.environ.get(
            "ZENITH_SCHEDULER_URL", 
            "http://localhost:8080"
        )
        self.timeout = timeout
        self.retry_count = retry_count
        self._session = requests.Session()
    
    def _request(
        self, 
        method: str, 
        endpoint: str, 
        json: Optional[dict] = None,
    ) -> dict:
        """Make API request with retries."""
        url = f"{self.base_url}{endpoint}"
        
        for attempt in range(self.retry_count):
            try:
                response = self._session.request(
                    method=method,
                    url=url,
                    json=json,
                    timeout=self.timeout,
                )
                
                if response.status_code >= 400:
                    error_msg = response.text
                    try:
                        error_data = response.json()
                        error_msg = error_data.get('message', error_msg)
                    except:
                        pass
                    raise SchedulerError(f"API error ({response.status_code}): {error_msg}")
                
                return response.json() if response.text else {}
                
            except requests.exceptions.ConnectionError as e:
                if attempt == self.retry_count - 1:
                    raise SchedulerError(f"Connection failed: {e}")
                time.sleep(1 * (attempt + 1))
            except requests.exceptions.Timeout:
                if attempt == self.retry_count - 1:
                    raise SchedulerError("Request timed out")
                time.sleep(1 * (attempt + 1))
        
        raise SchedulerError("Max retries exceeded")
    
    def health(self) -> bool:
        """Check if scheduler is healthy."""
        try:
            response = self._session.get(
                f"{self.base_url}/health",
                timeout=5.0,
            )
            return response.status_code == 200
        except:
            return False
    
    def submit(self, config: JobConfig) -> Job:
        """
        Submit a new job.
        
        Args:
            config: Job configuration
            
        Returns:
            Submitted job object
            
        Raises:
            SchedulerError: If submission fails
        """
        payload = {
            "name": config.name,
            "command": config.command,
            "arguments": config.arguments,
            "environment": config.environment,
            "working_directory": config.working_directory,
            "gpu_count": config.gpu_count,
            "cpu_cores": config.cpu_cores,
            "memory_mb": config.memory_mb,
            "priority": config.priority,
            "gang_schedule": config.gang_schedule,
            "user_id": config.user_id,
            "project_id": config.project_id,
        }
        
        response = self._request("POST", "/api/v1/jobs", json=payload)
        return Job.from_response(response)
    
    def get(self, job_id: str) -> Optional[Job]:
        """
        Get job status.
        
        Args:
            job_id: Job ID to query
            
        Returns:
            Job object or None if not found
        """
        try:
            response = self._request("GET", f"/api/v1/jobs/{job_id}")
            if response.get('state') == 'NOT_FOUND':
                return None
            return Job.from_response(response)
        except SchedulerError:
            return None
    
    def cancel(self, job_id: str) -> bool:
        """
        Cancel a job.
        
        Args:
            job_id: Job ID to cancel
            
        Returns:
            True if cancellation succeeded
        """
        try:
            response = self._request("DELETE", f"/api/v1/jobs/{job_id}")
            return response.get('status') == 'success'
        except SchedulerError:
            return False
    
    def list_jobs(self) -> List[Job]:
        """
        List all jobs.
        
        Returns:
            List of Job objects
        """
        response = self._request("GET", "/api/v1/jobs")
        return [Job.from_response(j) for j in response]
    
    def cluster_status(self) -> ClusterStatus:
        """
        Get cluster status.
        
        Returns:
            ClusterStatus object
        """
        response = self._request("GET", "/api/v1/cluster/status")
        return ClusterStatus.from_response(response)
    
    def list_nodes(self) -> List[Node]:
        """
        List cluster nodes.
        
        Returns:
            List of Node objects
        """
        response = self._request("GET", "/api/v1/nodes")
        return [Node.from_response(n) for n in response]
    
    def wait(
        self, 
        job_id: str, 
        timeout: Optional[float] = None,
        poll_interval: float = 5.0,
        callback: Optional[Callable[[Job], None]] = None,
    ) -> Job:
        """
        Wait for job to complete.
        
        Args:
            job_id: Job ID to wait for
            timeout: Maximum wait time in seconds (None = infinite)
            poll_interval: Time between status checks
            callback: Optional function to call on each poll
            
        Returns:
            Final job state
            
        Raises:
            SchedulerError: If timeout exceeded or job not found
        """
        start_time = time.time()
        terminal_states = {
            JobState.COMPLETED, 
            JobState.FAILED, 
            JobState.CANCELLED,
            JobState.TIMEOUT,
        }
        
        while True:
            job = self.get(job_id)
            
            if job is None:
                raise SchedulerError(f"Job {job_id} not found")
            
            if callback:
                callback(job)
            
            if job.state in terminal_states:
                return job
            
            if timeout and (time.time() - start_time) > timeout:
                raise SchedulerError(f"Timeout waiting for job {job_id}")
            
            time.sleep(poll_interval)


# Default client instance
_default_client: Optional[SchedulerClient] = None


def get_client() -> SchedulerClient:
    """Get or create default scheduler client."""
    global _default_client
    if _default_client is None:
        _default_client = SchedulerClient()
    return _default_client


def set_scheduler_url(url: str):
    """Set the scheduler URL for the default client."""
    global _default_client
    _default_client = SchedulerClient(base_url=url)


# ============================================================================
# Decorator API
# ============================================================================

def job(
    gpus: int = 0,
    cpus: int = 1,
    memory: str = "4GB",
    priority: int = 50,
    gang_schedule: bool = False,
    **kwargs,
):
    """
    Decorator to mark a function as a Zenith job.
    
    The decorated function can be submitted to the scheduler
    using zenith.submit().
    
    Args:
        gpus: Number of GPUs required
        cpus: Number of CPU cores
        memory: Memory requirement (e.g., "8GB", "16GB")
        priority: Job priority (higher = more important)
        gang_schedule: If True, all resources must be available together
        
    Example:
        >>> @zenith.job(gpus=4, memory="32GB")
        ... def train_model(epochs=10):
        ...     # Training code here
        ...     pass
        
        >>> job_id = zenith.submit(train_model, epochs=100)
    """
    # Parse memory string
    memory_mb = 4096
    if isinstance(memory, str):
        memory = memory.upper().strip()
        if memory.endswith("GB"):
            memory_mb = int(float(memory[:-2]) * 1024)
        elif memory.endswith("MB"):
            memory_mb = int(float(memory[:-2]))
        elif memory.endswith("TB"):
            memory_mb = int(float(memory[:-2]) * 1024 * 1024)
    elif isinstance(memory, (int, float)):
        memory_mb = int(memory)
    
    def decorator(func):
        @functools.wraps(func)
        def wrapper(*args, **call_kwargs):
            return func(*args, **call_kwargs)
        
        # Attach job configuration
        wrapper._zenith_job_config = {
            "gpu_count": gpus,
            "cpu_cores": cpus,
            "memory_mb": memory_mb,
            "priority": priority,
            "gang_schedule": gang_schedule,
            **kwargs,
        }
        wrapper._zenith_original_func = func
        
        return wrapper
    
    return decorator


def submit(
    func: Callable,
    *args,
    scheduler_url: Optional[str] = None,
    wait: bool = False,
    **kwargs,
) -> str:
    """
    Submit a job to the scheduler.
    
    Args:
        func: Function decorated with @zenith.job
        *args: Arguments to pass to the function
        scheduler_url: Optional scheduler URL (uses default if not set)
        wait: If True, wait for job completion
        **kwargs: Keyword arguments to pass
        
    Returns:
        Job ID
        
    Example:
        >>> job_id = zenith.submit(train_model, epochs=100)
        >>> print(f"Submitted: {job_id}")
    """
    if not hasattr(func, '_zenith_job_config'):
        raise ValueError(
            f"Function {func.__name__} is not a Zenith job. "
            "Decorate it with @zenith.job() first."
        )
    
    config = func._zenith_job_config
    original_func = func._zenith_original_func
    
    # Build command
    # For local execution, we run Python with the function
    # In production, this would serialize and ship the function
    import inspect
    module = inspect.getmodule(original_func)
    module_name = module.__name__ if module else "__main__"
    func_name = original_func.__name__
    
    # Build command line
    command = f"python -c \"from {module_name} import {func_name}; {func_name}()\""
    
    job_config = JobConfig(
        name=func_name,
        command=command,
        gpu_count=config.get("gpu_count", 0),
        cpu_cores=config.get("cpu_cores", 1),
        memory_mb=config.get("memory_mb", 4096),
        priority=config.get("priority", 50),
        gang_schedule=config.get("gang_schedule", False),
        user_id=os.environ.get("USER", "default"),
        project_id=os.environ.get("ZENITH_PROJECT", "default"),
    )
    
    try:
        client = SchedulerClient(base_url=scheduler_url) if scheduler_url else get_client()
        
        if not client.health():
            print(f"[zenith] Scheduler not available, running locally...")
            result = original_func(*args, **kwargs)
            return f"local-{id(result)}"
        
        job = client.submit(job_config)
        print(f"[zenith] Submitted job: {job.job_id} ({job.name})")
        
        if wait:
            print(f"[zenith] Waiting for completion...")
            final_job = client.wait(job.job_id, callback=lambda j: print(f"[zenith] State: {j.state.value}"))
            print(f"[zenith] Job {job.job_id} completed with state: {final_job.state.value}")
        
        return job.job_id
        
    except SchedulerError as e:
        print(f"[zenith] Scheduler error: {e}")
        print(f"[zenith] Falling back to local execution...")
        result = original_func(*args, **kwargs)
        return f"local-{id(result)}"


def status(job_id: Optional[str] = None) -> Optional[Job]:
    """
    Check job status.
    
    Args:
        job_id: Job ID to check
        
    Returns:
        Job object or None
    """
    if job_id is None:
        print("[zenith] No job ID provided")
        return None
    
    if job_id.startswith("local-"):
        print(f"[zenith] Job {job_id} was run locally (no tracking)")
        return None
    
    try:
        client = get_client()
        return client.get(job_id)
    except SchedulerError as e:
        print(f"[zenith] Error: {e}")
        return None


def cancel(job_id: str) -> bool:
    """
    Cancel a running job.
    
    Args:
        job_id: Job ID to cancel
        
    Returns:
        True if cancellation succeeded
    """
    if job_id.startswith("local-"):
        print(f"[zenith] Cannot cancel local job {job_id}")
        return False
    
    try:
        client = get_client()
        return client.cancel(job_id)
    except SchedulerError as e:
        print(f"[zenith] Error: {e}")
        return False


def cluster_info():
    """Print cluster information."""
    try:
        client = get_client()
        
        if not client.health():
            print("[zenith] Scheduler not available")
            return
        
        status = client.cluster_status()
        nodes = client.list_nodes()
        
        print("=" * 50)
        print("         ZENITH CLUSTER STATUS")
        print("=" * 50)
        print(f"  Nodes:      {status.healthy_nodes}/{status.total_nodes} healthy")
        print(f"  GPUs:       {status.available_gpus}/{status.total_gpus} available")
        print(f"  Jobs:       {status.running_jobs} running, {status.queued_jobs} queued")
        print("-" * 50)
        
        if nodes:
            print("  NODES:")
            for node in nodes:
                gpu_str = f"{node.available_gpus}/{node.total_gpus} GPUs"
                print(f"    {node.hostname}: {node.health} ({gpu_str})")
        
        print("=" * 50)
        
    except SchedulerError as e:
        print(f"[zenith] Error: {e}")


# Export public API
__all__ = [
    # Data classes
    "JobConfig",
    "Job",
    "JobState",
    "ClusterStatus",
    "Node",
    
    # Client
    "SchedulerClient",
    "SchedulerError",
    "get_client",
    "set_scheduler_url",
    
    # High-level API
    "job",
    "submit",
    "status",
    "cancel",
    "cluster_info",
]
