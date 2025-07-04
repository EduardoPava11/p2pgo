# Neural Network Training User Flow

This document describes the complete user flow for training neural networks from SGF files in P2P Go.

## Features Implemented

### 1. SGF Upload System
- **Batch Upload**: Select multiple SGF files or entire directories
- **Direct Paste**: Paste SGF content directly into the UI
- **Progress Tracking**: Visual progress bar for batch uploads
- **Error Handling**: Clear error messages for invalid SGF files

### 2. Game Management
- **Game Library**: Browse and filter uploaded games
- **Filtering Options**:
  - By move count (min/max)
  - By Ko situations (with/without)
  - By game result
- **Batch Selection**: Select multiple games for training
- **Game Details**: View move count, result, and Ko information

### 3. Ko Situation Handling
- **Automatic Detection**: Analyzes all games for Ko situations
- **Ko Statistics**: Shows percentage of games with Ko
- **Detailed Analysis**: Lists all Ko situations with move numbers
- **Synthetic Generation**: Creates Ko patterns when none exist in data

### 4. Neural Network Visualization
- **Real-time Overlay**: Shows neural net predictions during gameplay
- **Multiple Modes**:
  - Heat map: Move probability visualization
  - Top moves: Numbered best move predictions
  - Influence map: Territory control visualization
  - Combined: All visualizations together
- **Win Probability**: Real-time win chance display
- **Adjustable Settings**: Transparency, prediction display options

### 5. Error Logging System
- **Comprehensive Logging**: Tracks all errors, warnings, and info
- **Filtering**: View logs by severity level
- **Search**: Find specific errors quickly
- **Export**: Save logs to file for debugging
- **Persistent Storage**: Logs saved to disk automatically

## User Flow

### Step 1: Access Neural Training
1. Click "🧠 Neural Training" button in the top menu bar
2. Neural Training window opens with multiple tabs

### Step 2: Upload SGF Files
1. Go to "Upload SGF" tab
2. Choose upload method:
   - Click "Select Files" to choose individual SGF files
   - Click "Select Folder" to import entire directories
   - Paste SGF content directly in the text area
3. Click "Upload All" to process files
4. See success messages for each uploaded game

### Step 3: Manage Games
1. Switch to "Game Library" tab
2. Apply filters if needed:
   - Set minimum/maximum move counts
   - Filter for games with/without Ko
3. Select games for training:
   - Click "Select All" for all filtered games
   - Or manually check individual games
4. Selected count shows at the top

### Step 4: Analyze Ko Situations
1. Go to "Ko Analysis" tab
2. Click "Analyze All Games" button
3. View statistics:
   - Total games analyzed
   - Games containing Ko situations
   - Total Ko situations found
4. If no Ko found:
   - Click "Generate Ko Training Patterns"
   - 5 synthetic Ko situations are created

### q: Create Training Data
1. Switch to "Training" tab
2. Configure training options:
   - Include Ko positions: Yes/No
   - Data augmentation: Yes/No
   - Validation split: 10-40%
   - Batch size: Adjustable
3. Click "Create Training Dataset"
4. Dataset is created and listed

### Step 6: Neural Visualization During Play
1. Start or join a game
2. Neural controls panel appears on the right
3. Enable overlay with checkbox
4. Choose visualization mode:
   - Heat Map for move probabilities
   - Top Moves for numbered predictions
   - Influence for territory control
   - Combined for all visualizations
5. Adjust transparency slider for visibility
6. Watch win probability bar update in real-time

### Step 7: Monitor with Error Logs
1. Click "📋 Error Log" in top menu
2. Error Log window opens
3. Filter by severity if needed
4. Search for specific errors
5. Export logs for detailed analysis

## Error Handling

### Common Issues and Solutions

1. **"No valid SGF files found"**
   - Ensure files have .sgf extension
   - Check file format is valid SGF

2. **"Failed to parse SGF"**
   - Verify SGF syntax is correct
   - Check for corrupted files

3. **"No Ko situations found"**
   - Use "Generate Ko Training Patterns" feature
   - Upload games known to contain Ko

4. **"Insufficient training data"**
   - Upload more games (minimum 10 required)
   - Use data augmentation option

## Tips for Best Results

1. **Diverse Training Data**: Upload games from various skill levels
2. **Ko Situations**: Include games with Ko for better training
3. **Batch Processing**: Use folder upload for large datasets
4. **Regular Monitoring**: Check error logs for issues
5. **Visualization**: Use neural overlay to verify training effectiveness

## Technical Details

- SGF files are parsed and converted to game states
- Ko detection runs automatically on all uploaded games
- Training data includes board positions and move sequences
- Neural network uses dual policy/value architecture
- Error logging persists across sessions

## Future Enhancements

- Cloud training support
- Model sharing between players
- Advanced Ko pattern library
- Training progress visualization
- Model performance metrics