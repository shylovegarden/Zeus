#!/usr/bin/env python3
"""
Zeus Agent Loop: AI Self-Repair Without Human Intervention

This script implements the AI auto-repair pillar of Zeus's competitive advantage.
It:
1. Runs Zeus trust-gate verification on code
2. Parses JSON gap analysis with distance-to-proof metrics
3. Applies high-confidence repair candidates automatically
4. Re-verifies without human intervention
5. Loops until verified or max iterations reached

This enables AI agents to mathematically debug their own code without human oversight.
"""

import json
import subprocess
import sys
import re
from pathlib import Path
from typing import Dict, List, Optional
from dataclasses import dataclass


@dataclass
class RepairCandidate:
    """A repair candidate from Zeus verification result."""
    line: int
    fix: str
    confidence: float


@dataclass
class GapAnalysis:
    """Gap analysis from Zeus verification result."""
    missing_invariant: str
    suggested_fix: str
    line: Optional[int]


@dataclass
class VerificationResult:
    """Rich verification result from Zeus trust-gate."""
    function: str
    status: str
    distance_to_proof: int
    gap_analysis: List[GapAnalysis]
    repair_candidates: List[RepairCandidate]
    security_properties: Dict[str, bool]


class ZeusAgentLoop:
    """AI self-repair loop for Zeus verification."""
    
    def __init__(
        self,
        source_path: str,
        max_iterations: int = 10,
        min_confidence: float = 0.85,
        zeus_command: str = "cargo run -- trust-gate"
    ):
        self.source_path = Path(source_path)
        self.max_iterations = max_iterations
        self.min_confidence = min_confidence
        self.zeus_command = zeus_command
        self.iteration = 0
        self.history = []
        
    def run_verification(self) -> VerificationResult:
        """Run Zeus trust-gate and parse JSON output."""
        cmd = f"{self.zeus_command} {self.source_path} --json"
        
        try:
            result = subprocess.run(
                cmd,
                shell=True,
                cwd="/Users/shy/Developer/ZEUS/zeus_compiler",
                capture_output=True,
                text=True,
                timeout=30
            )
            
            if result.returncode != 0:
                print(f"Zeus verification failed: {result.stderr}")
                return None
                
            data = json.loads(result.stdout)
            
            # Parse gap analysis
            gap_analysis = [
                GapAnalysis(
                    missing_invariant=g["missing_invariant"],
                    suggested_fix=g["suggested_fix"],
                    line=g.get("line")
                )
                for g in data.get("gap_analysis", [])
            ]
            
            # Parse repair candidates
            repair_candidates = [
                RepairCandidate(
                    line=r["line"],
                    fix=r["fix"],
                    confidence=r["confidence"]
                )
                for r in data.get("repair_candidates", [])
            ]
            
            return VerificationResult(
                function=data.get("function", "unknown"),
                status=data.get("status", "unknown"),
                distance_to_proof=data.get("distance_to_proof", 0),
                gap_analysis=gap_analysis,
                repair_candidates=repair_candidates,
                security_properties=data.get("security_properties", {})
            )
            
        except subprocess.TimeoutExpired:
            print("Zeus verification timed out")
            return None
        except json.JSONDecodeError as e:
            print(f"Failed to parse Zeus JSON output: {e}")
            return None
        except Exception as e:
            print(f"Unexpected error: {e}")
            return None
    
    def apply_repair(self, repair: RepairCandidate) -> bool:
        """Apply a repair candidate to the source file."""
        try:
            with open(self.source_path, 'r') as f:
                lines = f.readlines()
            
            if repair.line <= 0 or repair.line > len(lines):
                print(f"Invalid line number: {repair.line}")
                return False
            
            # Replace the line
            lines[repair.line - 1] = repair.fix + "\n"
            
            with open(self.source_path, 'w') as f:
                f.writelines(lines)
            
            print(f"✓ Applied repair at line {repair.line}: {repair.fix[:50]}...")
            return True
            
        except Exception as e:
            print(f"Failed to apply repair: {e}")
            return False
    
    def run(self) -> bool:
        """Run the AI self-repair loop."""
        print("🤖 Zeus Agent Loop: AI Self-Repair")
        print("=" * 50)
        print(f"Source: {self.source_path}")
        print(f"Max iterations: {self.max_iterations}")
        print(f"Min confidence: {self.min_confidence}")
        print()
        
        while self.iteration < self.max_iterations:
            self.iteration += 1
            print(f"\n--- Iteration {self.iteration}/{self.max_iterations} ---")
            
            # Run verification
            result = self.run_verification()
            
            if result is None:
                print("❌ Verification failed - aborting")
                return False
            
            print(f"Status: {result.status}")
            print(f"Distance to proof: {result.distance_to_proof}")
            print(f"Security properties: {result.security_properties}")
            
            # Check if verified
            if result.status == "verified":
                print("\n✅ Code verified successfully!")
                self.history.append({
                    "iteration": self.iteration,
                    "status": "verified",
                    "distance_to_proof": result.distance_to_proof
                })
                return True
            
            # Get high-confidence repairs
            high_confidence_repairs = [
                r for r in result.repair_candidates
                if r.confidence >= self.min_confidence
            ]
            
            if not high_confidence_repairs:
                print("⚠️  No high-confidence repairs available")
                print("Gap analysis:")
                for gap in result.gap_analysis:
                    print(f"  - {gap.missing_invariant}: {gap.suggested_fix}")
                self.history.append({
                    "iteration": self.iteration,
                    "status": "no_repairs",
                    "distance_to_proof": result.distance_to_proof
                })
                break
            
            # Apply the best repair
            best_repair = max(high_confidence_repairs, key=lambda r: r.confidence)
            print(f"Applying repair (confidence: {best_repair.confidence:.2f})")
            
            if not self.apply_repair(best_repair):
                print("❌ Failed to apply repair - aborting")
                return False
            
            self.history.append({
                "iteration": self.iteration,
                "status": "repaired",
                "distance_to_proof": result.distance_to_proof,
                "repair_applied": best_repair.fix
            })
        
        print(f"\n⚠️  Max iterations reached without verification")
        print(f"Final distance to proof: {result.distance_to_proof if result else 'unknown'}")
        return False
    
    def print_history(self):
        """Print the repair history."""
        print("\n📊 Repair History:")
        print("-" * 50)
        for entry in self.history:
            print(f"Iteration {entry['iteration']}: {entry['status']}")
            if 'distance_to_proof' in entry:
                print(f"  Distance to proof: {entry['distance_to_proof']}")
            if 'repair_applied' in entry:
                print(f"  Repair: {entry['repair_applied'][:50]}...")


def main():
    """Main entry point."""
    if len(sys.argv) < 2:
        print("Usage: python zeus_agent_loop.py <source.zs> [max_iterations] [min_confidence]")
        sys.exit(1)
    
    source_path = sys.argv[1]
    max_iterations = int(sys.argv[2]) if len(sys.argv) > 2 else 10
    min_confidence = float(sys.argv[3]) if len(sys.argv) > 3 else 0.85
    
    if not Path(source_path).exists():
        print(f"Error: Source file not found: {source_path}")
        sys.exit(1)
    
    # Create and run agent loop
    agent = ZeusAgentLoop(
        source_path=source_path,
        max_iterations=max_iterations,
        min_confidence=min_confidence
    )
    
    success = agent.run()
    agent.print_history()
    
    sys.exit(0 if success else 1)


if __name__ == "__main__":
    main()
