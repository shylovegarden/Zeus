const { ethers } = require("hardhat");

async function main() {
    const [deployer] = await ethers.getSigners();
    
    console.log("Deploying contracts with account:", deployer.address);
    console.log("Account balance:", (await deployer.getBalance()).toString());
    
    // Deploy mock tokens
    const Token = await ethers.getContractFactory("MockERC20");
    
    const tokenA = await Token.deploy("Token A", "TKA", ethers.utils.parseEther("1000000"));
    await tokenA.deployed();
    console.log("TokenA deployed to:", tokenA.address);
    
    const tokenB = await Token.deploy("Token B", "TKB", ethers.utils.parseEther("1000000"));
    await tokenB.deployed();
    console.log("TokenB deployed to:", tokenB.address);
    
    // Deploy AMM
    const AMM = await ethers.getContractFactory("ZeusAMM");
    const amm = await AMM.deploy(tokenA.address, tokenB.address);
    await amm.deployed();
    console.log("ZeusAMM deployed to:", amm.address);
    
    // Add initial liquidity
    const liquidityAmount = ethers.utils.parseEther("10000");
    
    await tokenA.approve(amm.address, liquidityAmount);
    await tokenB.approve(amm.address, liquidityAmount);
    
    await amm.addLiquidity(liquidityAmount, liquidityAmount, 0);
    console.log("Initial liquidity added:", liquidityAmount.toString());
    
    console.log("\nDeployment complete!");
    console.log("TokenA:", tokenA.address);
    console.log("TokenB:", tokenB.address);
    console.log("AMM:", amm.address);
}

main()
    .then(() => process.exit(0))
    .catch((error) => {
        console.error(error);
        process.exit(1);
    });
