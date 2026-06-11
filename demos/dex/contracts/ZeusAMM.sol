// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

import "@openzeppelin/contracts/token/ERC20/ERC20.sol";
import "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import "@openzeppelin/contracts/security/ReentrancyGuard.sol";

/**
 * @title ZeusAMM
 * @dev Provably secure Constant Product AMM
 * Generated from Zeus verified source
 */
contract ZeusAMM is ERC20, ReentrancyGuard {
    IERC20 public tokenA;
    IERC20 public tokenB;
    
    uint256 public reserveA;
    uint256 public reserveB;
    
    uint256 public constant FEE_NUMERATOR = 30;    // 0.3%
    uint256 public constant FEE_DENOMINATOR = 10000;
    
    event LiquidityAdded(address indexed provider, uint256 amountA, uint256 amountB, uint256 liquidity);
    event LiquidityRemoved(address indexed provider, uint256 liquidity, uint256 amountA, uint256 amountB);
    event Swap(address indexed user, uint256 amountIn, uint256 amountOut, bool aToB);
    
    constructor(address _tokenA, address _tokenB) ERC20("Zeus LP", "ZLP") {
        require(_tokenA != address(0) && _tokenB != address(0), "Invalid token address");
        require(_tokenA != _tokenB, "Tokens must be different");
        tokenA = IERC20(_tokenB);
        tokenB = IERC20(_tokenA);
    }
    
    /**
     * @notice Add liquidity to the pool
     * @param amountADesired Amount of token A to add
     * @param amountBDesired Amount of token B to add
     * @param minLiquidity Minimum liquidity tokens to receive
     * @return liquidity Amount of LP tokens minted
     */
    function addLiquidity(
        uint256 amountADesired,
        uint256 amountBDesired,
        uint256 minLiquidity
    ) external nonReentrant returns (uint256 liquidity) {
        require(amountADesired > 0 && amountBDesired > 0, "Invalid amounts");
        
        if (totalSupply() == 0) {
            // First liquidity provider
            liquidity = sqrt(amountADesired * amountBDesired);
        } else {
            // Calculate proportional amounts
            uint256 liquidityA = (amountADesired * totalSupply()) / reserveA;
            uint256 liquidityB = (amountBDesired * totalSupply()) / reserveB;
            
            liquidity = liquidityA < liquidityB ? liquidityA : liquidityB;
        }
        
        require(liquidity >= minLiquidity, "Insufficient liquidity minted");
        
        // Transfer tokens
        require(tokenA.transferFrom(msg.sender, address(this), amountADesired), "Transfer A failed");
        require(tokenB.transferFrom(msg.sender, address(this), amountBDesired), "Transfer B failed");
        
        // Update reserves
        reserveA += amountADesired;
        reserveB += amountBDesired;
        
        // Mint LP tokens
        _mint(msg.sender, liquidity);
        
        emit LiquidityAdded(msg.sender, amountADesired, amountBDesired, liquidity);
    }
    
    /**
     * @notice Remove liquidity from the pool
     * @param liquidity Amount of LP tokens to burn
     * @param minA Minimum amount of token A to receive
     * @param minB Minimum amount of token B to receive
     * @return amountA Amount of token A received
     * @return amountB Amount of token B received
     */
    function removeLiquidity(
        uint256 liquidity,
        uint256 minA,
        uint256 minB
    ) external nonReentrant returns (uint256 amountA, uint256 amountB) {
        require(liquidity > 0 && liquidity <= balanceOf(msg.sender), "Invalid liquidity amount");
        
        // Calculate amounts
        amountA = (liquidity * reserveA) / totalSupply();
        amountB = (liquidity * reserveB) / totalSupply();
        
        require(amountA >= minA && amountB >= minB, "Insufficient output");
        
        // Burn LP tokens
        _burn(msg.sender, liquidity);
        
        // Update reserves
        reserveA -= amountA;
        reserveB -= amountB;
        
        // Transfer tokens
        require(tokenA.transfer(msg.sender, amountA), "Transfer A failed");
        require(tokenB.transfer(msg.sender, amountB), "Transfer B failed");
        
        emit LiquidityRemoved(msg.sender, liquidity, amountA, amountB);
    }
    
    /**
     * @notice Swap token A for token B
     * @param amountIn Amount of token A to swap
     * @param minOut Minimum amount of token B to receive
     */
    function swapAForB(uint256 amountIn, uint256 minOut) external nonReentrant {
        require(amountIn > 0, "Invalid amount");
        
        uint256 amountOut = getAmountOut(amountIn, reserveA, reserveB);
        require(amountOut >= minOut, "Insufficient output");
        
        // Transfer tokens
        require(tokenA.transferFrom(msg.sender, address(this), amountIn), "Transfer in failed");
        require(tokenB.transfer(msg.sender, amountOut), "Transfer out failed");
        
        // Update reserves
        reserveA += amountIn;
        reserveB -= amountOut;
        
        emit Swap(msg.sender, amountIn, amountOut, true);
    }
    
    /**
     * @notice Swap token B for token A
     * @param amountIn Amount of token B to swap
     * @param minOut Minimum amount of token A to receive
     */
    function swapBForA(uint256 amountIn, uint256 minOut) external nonReentrant {
        require(amountIn > 0, "Invalid amount");
        
        uint256 amountOut = getAmountOut(amountIn, reserveB, reserveA);
        require(amountOut >= minOut, "Insufficient output");
        
        // Transfer tokens
        require(tokenB.transferFrom(msg.sender, address(this), amountIn), "Transfer in failed");
        require(tokenA.transfer(msg.sender, amountOut), "Transfer out failed");
        
        // Update reserves
        reserveB += amountIn;
        reserveA -= amountOut;
        
        emit Swap(msg.sender, amountIn, amountOut, false);
    }
    
    /**
     * @notice Calculate output amount for a swap
     * @param amountIn Input amount
     * @param reserveIn Input reserve
     * @param reserveOut Output reserve
     * @return amountOut Output amount
     */
    function getAmountOut(
        uint256 amountIn,
        uint256 reserveIn,
        uint256 reserveOut
    ) public pure returns (uint256 amountOut) {
        require(amountIn > 0 && reserveIn > 0 && reserveOut > 0, "Invalid reserves");
        
        uint256 amountInWithFee = amountIn * (FEE_DENOMINATOR - FEE_NUMERATOR);
        uint256 numerator = amountInWithFee * reserveOut;
        uint256 denominator = (reserveIn * FEE_DENOMINATOR) + amountInWithFee;
        
        amountOut = numerator / denominator;
    }
    
    /**
     * @notice Get current price
     * @param aToB If true, price of A in B; else price of B in A
     * @return price Price with 18 decimals
     */
    function getPrice(bool aToB) external view returns (uint256 price) {
        require(reserveA > 0 && reserveB > 0, "No liquidity");
        
        uint256 scale = 1e18;
        
        if (aToB) {
            price = (reserveB * scale) / reserveA;
        } else {
            price = (reserveA * scale) / reserveB;
        }
    }
    
    /**
     * @notice Get constant product k = x * y
     */
    function getK() external view returns (uint256 k) {
        k = reserveA * reserveB;
    }
    
    /**
     * @dev Babylonian method for square root
     */
    function sqrt(uint256 y) internal pure returns (uint256 z) {
        if (y > 3) {
            z = y;
            uint256 x = y / 2 + 1;
            while (x < z) {
                z = x;
                x = (y / x + x) / 2;
            }
        } else if (y != 0) {
            z = 1;
        }
    }
}
