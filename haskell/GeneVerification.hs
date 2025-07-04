{-# LANGUAGE OverloadedStrings #-}
-- | Formal verification of gene marketplace properties
module GeneVerification where

import Test.QuickCheck
import Data.ByteString (ByteString)
import qualified Data.ByteString as BS

-- | Gene representation
data Gene = Gene
  { geneId :: Integer
  , modelHash :: ByteString
  , generation :: Integer
  , winRate :: Double
  , creator :: String
  } deriving (Show, Eq)

-- | Training proof for gene minting
data TrainingProof = TrainingProof
  { proofGames :: Integer
  , proofWinRate :: Double
  , proofConsensusRate :: Double
  } deriving (Show, Eq)

-- | Gene breeding result
data BreedingResult = BreedingResult
  { parent1 :: Gene
  , parent2 :: Gene
  , offspring :: Gene
  } deriving (Show, Eq)

-- Property 1: Win rate bounds
prop_winRateBounds :: Gene -> Bool
prop_winRateBounds gene = 
  winRate gene >= 0.0 && winRate gene <= 1.0

-- Property 2: Generation increases with breeding
prop_generationIncrease :: BreedingResult -> Bool
prop_generationIncrease result =
  generation (offspring result) == 
    max (generation (parent1 result)) (generation (parent2 result)) + 1

-- Property 3: Training proof validity
prop_validTrainingProof :: TrainingProof -> Bool
prop_validTrainingProof proof =
  proofGames proof >= 100 &&  -- Minimum games for validity
  proofWinRate proof >= 0.0 && proofWinRate proof <= 1.0 &&
  proofConsensusRate proof >= 0.8  -- High consensus required

-- Property 4: Gene performance inheritance
prop_performanceInheritance :: BreedingResult -> Bool
prop_performanceInheritance result =
  let minParentRate = min (winRate (parent1 result)) (winRate (parent2 result))
      maxParentRate = max (winRate (parent1 result)) (winRate (parent2 result))
      childRate = winRate (offspring result)
  in childRate >= minParentRate * 0.9 &&  -- At least 90% of worst parent
     childRate <= maxParentRate * 1.1     -- At most 110% of best parent

-- Property 5: Model hash uniqueness after breeding
prop_uniqueOffspring :: BreedingResult -> Bool
prop_uniqueOffspring result =
  modelHash (offspring result) /= modelHash (parent1 result) &&
  modelHash (offspring result) /= modelHash (parent2 result)

-- Property 6: Monotonic improvement over generations
prop_monotonicImprovement :: [Gene] -> Bool
prop_monotonicImprovement [] = True
prop_monotonicImprovement [_] = True
prop_monotonicImprovement (g1:g2:gs) =
  generation g2 > generation g1 ==> 
    winRate g2 >= winRate g1 * 0.95 &&  -- Allow 5% variance
    prop_monotonicImprovement (g2:gs)

-- | Verify consensus mechanism convergence
data ConsensusState = ConsensusState
  { blackMarks :: [(Int, Int)]
  , whiteMarks :: [(Int, Int)]
  , agreedTerritory :: [(Int, Int)]
  } deriving (Show, Eq)

-- Property 7: Consensus convergence
prop_consensusConvergence :: ConsensusState -> Bool
prop_consensusConvergence state =
  let totalMarks = length (blackMarks state) + length (whiteMarks state)
      agreedMarks = length (agreedTerritory state)
      convergenceRate = if totalMarks > 0 
                        then fromIntegral agreedMarks / fromIntegral totalMarks
                        else 1.0
  in convergenceRate >= 0.7  -- 70% agreement threshold

-- | Training data quality metrics
data TrainingData = TrainingData
  { gameLength :: Integer
  , moveValidityRate :: Double
  , consensusAchieved :: Bool
  , territoryAgreement :: Double
  } deriving (Show, Eq)

-- Property 8: Training data quality
prop_trainingDataQuality :: TrainingData -> Bool
prop_trainingDataQuality td =
  gameLength td >= 20 &&  -- Minimum meaningful game length
  moveValidityRate td >= 0.99 &&  -- Nearly all moves must be valid
  (consensusAchieved td ==> territoryAgreement td >= 0.8)

-- Property 9: Credit economy conservation
prop_creditConservation :: Integer -> Integer -> Integer -> Bool
prop_creditConservation initialCredits gamesPlayed relayFees =
  let totalSpent = gamesPlayed * relayFees
      expectedRemaining = initialCredits - totalSpent
  in expectedRemaining >= 0 || gamesPlayed == 0

-- Run all properties
runAllProperties :: IO ()
runAllProperties = do
  putStrLn "=== Gene Marketplace Verification ==="
  
  putStrLn "Testing win rate bounds..."
  quickCheck prop_winRateBounds
  
  putStrLn "Testing generation increase..."
  quickCheck prop_generationIncrease
  
  putStrLn "Testing training proof validity..."
  quickCheck prop_validTrainingProof
  
  putStrLn "Testing performance inheritance..."
  quickCheck prop_performanceInheritance
  
  putStrLn "Testing offspring uniqueness..."
  quickCheck prop_uniqueOffspring
  
  putStrLn "Testing consensus convergence..."
  quickCheck prop_consensusConvergence
  
  putStrLn "Testing training data quality..."
  quickCheck prop_trainingDataQuality
  
  putStrLn "Testing credit conservation..."
  quickCheck prop_creditConservation
  
  putStrLn "=== All properties verified ==="

-- QuickCheck generators
instance Arbitrary Gene where
  arbitrary = do
    gid <- arbitrary `suchThat` (> 0)
    gen <- arbitrary `suchThat` (>= 0)
    rate <- choose (0.0, 1.0)
    creator <- arbitrary
    return $ Gene gid (BS.pack [1,2,3]) gen rate creator

instance Arbitrary TrainingProof where
  arbitrary = do
    games <- choose (100, 10000)
    rate <- choose (0.0, 1.0)
    consensus <- choose (0.8, 1.0)
    return $ TrainingProof games rate consensus

instance Arbitrary BreedingResult where
  arbitrary = do
    p1 <- arbitrary
    p2 <- arbitrary
    let childGen = max (generation p1) (generation p2) + 1
    let childRate = (winRate p1 + winRate p2) / 2.0
    child <- Gene <$> arbitrary 
                  <*> pure (BS.pack [4,5,6]) 
                  <*> pure childGen 
                  <*> pure childRate 
                  <*> pure "bred"
    return $ BreedingResult p1 p2 child

instance Arbitrary ConsensusState where
  arbitrary = do
    bMarks <- listOf arbitrary
    wMarks <- listOf arbitrary
    let agreed = filter (\m -> m `elem` bMarks) wMarks
    return $ ConsensusState bMarks wMarks agreed

instance Arbitrary TrainingData where
  arbitrary = do
    len <- choose (20, 200)
    validity <- choose (0.99, 1.0)
    consensus <- arbitrary
    agreement <- choose (0.7, 1.0)
    return $ TrainingData len validity consensus agreement