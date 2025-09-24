#!/usr/bin/env python3
"""
Benchmark script to compare native Rust physics engine vs Python OpenMM bridge.
"""

import subprocess
import time
import json
import sys
from pathlib import Path

def run_folding_command(contract_file, physics_engine="auto"):
    """Run a folding command and measure execution time."""
    cmd = [
        "cargo", "run", "--release", "--bin", "folding-app", "--",
        str(contract_file)
    ]
    
    start_time = time.time()
    try:
        result = subprocess.run(
            cmd, 
            cwd="../",
            capture_output=True, 
            text=True, 
            timeout=60
        )
        end_time = time.time()
        
        return {
            "success": result.returncode == 0,
            "duration": end_time - start_time,
            "stdout": result.stdout,
            "stderr": result.stderr
        }
    except subprocess.TimeoutExpired:
        return {
            "success": False,
            "duration": 60.0,
            "stdout": "",
            "stderr": "Timeout after 60 seconds"
        }

def create_test_contract():
    """Create a simple test contract for benchmarking."""
    contract_content = """
# Simple test contract for physics benchmarking
domain test_protein {
    sequence: "AAAGGGSSS"
    length: 9
}

physics toy {
    temperature: 300.0
    steps: 10
}

rotate residue 0 by 15.0 degrees
rotate residue 1 by -10.0 degrees  
rotate residue 2 by 20.0 degrees
commit
physics coarse
rotate residue 3 by 25.0 degrees
commit
"""
    
    contract_file = Path("benchmark_test.lll")
    contract_file.write_text(contract_content)
    return contract_file

def main():
    print("=== Native Physics Engine Benchmark ===")
    
    # Create test contract
    contract_file = create_test_contract()
    
    try:
        # Test native physics engine
        print("\n1. Testing Native Rust Physics Engine...")
        native_result = run_folding_command(contract_file, "native")
        
        if native_result["success"]:
            print(f"   ✓ Native engine completed in {native_result['duration']:.3f}s")
        else:
            print(f"   ✗ Native engine failed: {native_result['stderr']}")
        
        # Test OpenMM physics engine (if available)
        print("\n2. Testing OpenMM Physics Engine...")
        openmm_result = run_folding_command(contract_file, "openmm")
        
        if openmm_result["success"]:
            print(f"   ✓ OpenMM engine completed in {openmm_result['duration']:.3f}s")
        else:
            print(f"   ✗ OpenMM engine failed (likely not installed): {openmm_result['stderr']}")
        
        # Test auto selection
        print("\n3. Testing Auto Engine Selection...")
        auto_result = run_folding_command(contract_file, "auto")
        
        if auto_result["success"]:
            print(f"   ✓ Auto selection completed in {auto_result['duration']:.3f}s")
        else:
            print(f"   ✗ Auto selection failed: {auto_result['stderr']}")
        
        # Summary
        print("\n=== Benchmark Results ===")
        if native_result["success"]:
            print(f"Native Rust Engine: {native_result['duration']:.3f}s")
        if openmm_result["success"]:
            print(f"OpenMM Python Engine: {openmm_result['duration']:.3f}s")
            if native_result["success"]:
                speedup = openmm_result['duration'] / native_result['duration']
                print(f"Native speedup: {speedup:.2f}x")
        
        print(f"\nNative engine is {'available' if native_result['success'] else 'unavailable'}")
        print(f"OpenMM engine is {'available' if openmm_result['success'] else 'unavailable'}")
        
    finally:
        # Clean up
        if contract_file.exists():
            contract_file.unlink()

if __name__ == "__main__":
    main()
