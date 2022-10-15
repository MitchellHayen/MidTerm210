use rand::Rng;
use std::thread;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};

fn main()
{
    //creating shared memory space protected by mutex
    let grid = Arc::new(Mutex::new([['-' as char; 4]; 4]));

    let mut characterArray = vec![];
    //current x, current y, char symbol, previous x, previous y, has gold? (if gold, then boolean indicates if it's been captured already or not)
    let robot :(i8,i8,char,i8,i8,bool) = (3,3,'R',3,3,false);
    let bomb1 :(i8,i8,char,i8,i8,bool) = (2,0,'B',2,0,true);
    let bomb2 :(i8,i8,char,i8,i8,bool) = (0,3,'B',0,3,false);
    let mut gold1 :(i8,i8,char,i8,i8,bool) = (3,0,'G',3,0,false);
    let mut gold2 :(i8,i8,char,i8,i8,bool) = (0,2,'G',0,2,false);
    characterArray.push(robot);
    characterArray.push(bomb1);
    characterArray.push(bomb2);

    let mut end = false;
    //counters used for bomb movement
    let mut counter1 = 2;
    let mut counter2 = 0;
    let mut wizard_counter = 0;
    let mut wizard_shuffle = false;
    {
        //bringing in our shared memory space
        let grid = Arc::clone(&grid);
        //unlocking the shared memory space
        let mut grid1 = grid.lock().unwrap();
        grid1[gold1.0 as usize][gold1.1 as usize] = 'G';
        grid1[gold2.0 as usize][gold2.1 as usize] = 'G';
    }
    //shared memory space is now locked again since it went out of scope

    //main game loop
    loop
    {
        let bomb1_move = if counter1%4 == 0 {true} else {false};
        let bomb2_move = if counter2%4 == 0 {true} else {false};
        counter1 += 1;
        counter2 += 1;
        wizard_counter += 1;
        if wizard_counter%5 == 0 { wizard_shuffle = true};

        let mut temp_character_array = vec![];
        let mut character_array_clone = characterArray.clone();
        if wizard_shuffle == true
        {
            //clear board first
            let grid = Arc::clone(&grid);
            {
                let mut grid1 = grid.lock().unwrap();
                *grid1 = [['-' as char; 4]; 4];
            }
            println!("Wizard Shuffle!");
            let mut gold_qty = false; //if false, no gold has been picked up so 2 gold need to be placed
            //using character_array_clone -- if characterArray was used, it would be consumed and not usable again
            for mut character in character_array_clone
            {
                let grid = Arc::clone(&grid);
                //if robot has a gold piece, then we will only put 1 gold piece on the new board
                if character.2 == 'R' && character.5 == true { gold_qty = true}
                let mut rng = rand::thread_rng();
                let mut row = rng.gen_range(0, 4);
                let mut column = rng.gen_range(0, 4);
                loop
                {
                    //changing position until an open space is found
                    let mut grid1 = grid.lock().unwrap();
                    row = rng.gen_range(0, 4);
                    column = rng.gen_range(0, 4);
                    if grid1[row as usize][column as usize] == '-' {break;}
                }
                //an open space was found, now we can change the characters coordinates
                character.0 = row;
                character.1 = column;
                character.3 = row;
                character.4 = column;
                let mut grid1 = grid.lock().unwrap();
                grid1[character.0 as usize][character.1 as usize] = character.2;
                //add character to temp_character_array so that we can update characterArray later
                temp_character_array.push(character);
            }
            let grid = Arc::clone(&grid);
            let mut rng = rand::thread_rng();
            let mut row = rng.gen_range(0, 4);
            let mut column = rng.gen_range(0, 4);
            //gettings coords for gold piece
            loop
            {
                let grid1 = grid.lock().unwrap();
                row = rng.gen_range(0, 4);
                column = rng.gen_range(0, 4);
                if grid1[row as usize][column as usize] == '-' {break;}
            }

            {
                //setting gold piece but using a new scope so that the mutex locks when we're done editing it
                let mut grid1 = grid.lock().unwrap();
                grid1[row][column] = 'G';
            }
            //if no gold pieces were picked up yet, look for place for another gold piece
            if !gold_qty
            {
                let grid = Arc::clone(&grid);
                let mut rng = rand::thread_rng();
                let mut row = rng.gen_range(0, 4);
                let mut column = rng.gen_range(0, 4);
                loop
                {
                    let mut grid1 = grid.lock().unwrap();
                    row = rng.gen_range(0, 4);
                    column = rng.gen_range(0, 4);
                    if grid1[row as usize][column as usize] == '-' {break;}
                }

                let mut grid1 = grid.lock().unwrap();
                grid1[row][column] = 'G';
            }

            //updating characterArray with the new positions of all characters
            characterArray = temp_character_array;
        }
        //resetting so that the board doesn't shuffle every round
        wizard_shuffle = false;

        //creating channels to bring information out of threads back to main thread to be used
        let (end_tx, end_rx) = mpsc::channel();
        let (wizard_tx, wizard_rx) = mpsc::channel();
        let mut handles = vec![];
        let mut tempCharacterArray = vec![];
        for mut character in characterArray.clone()
        {
            //saving current coordinates to change current space to blank after character moves
            let prev_x = character.0 as usize;
            let prev_y = character.1 as usize;
            let mut temp_character = character;
            let (tx, rx) = mpsc::channel();
            let temp_end_tx = end_tx.clone();
            let temp_wizard_tx = wizard_tx.clone();
            let grid = Arc::clone(&grid);
            //creating a thread for each character
            let handle = thread::spawn(move ||
                {
                    let mut end1 = false;
                    loop
                    {
                        let mut grid1 = grid.lock().unwrap();
                        temp_character = character_move(character);
                        if character.2 == 'B' && character.5 == true && !bomb1_move { break };
                        if character.2 == 'B' && character.5 == false && !bomb2_move { break };
                        if grid1[temp_character.0 as usize][temp_character.1 as usize] == '-'
                        {
                            //found a place to move so we can update characters coordinates
                            character = temp_character;
                            break;
                        }
                        else if grid1[temp_character.0 as usize][temp_character.1 as usize] == 'G' && character.2 == 'R'
                        {
                            //already has a gold piece
                            if character.5 == true
                            {
                                println!("Robot wins!");
                                end1 = true;
                                character = temp_character;
                                break;
                            }
                                //does not have any gold
                            else
                            {
                                character = temp_character;
                                character.5 = true;
                                wizard_shuffle = true;
                                break;
                            }
                        }
                        else if grid1[temp_character.0 as usize][temp_character.1 as usize] == 'R' && character.2 == 'B'
                        {
                            println!("Robot loses!");
                            character = temp_character;
                            end1 = true;
                            break;
                        }
                    }
                    let mut grid1 = grid.lock().unwrap();
                    //sending information to be received in main thread
                    temp_end_tx.send(end1).unwrap();
                    temp_wizard_tx.send(wizard_shuffle).unwrap();
                    let temp_tx = tx.clone();
                    temp_tx.send(character).unwrap();
                    grid1[prev_x][prev_y] = '-';
                    grid1[character.0 as usize][character.1 as usize] = character.2;
                });
            let received = rx.recv().unwrap();
            tempCharacterArray.push(received);
            handles.push(handle);
        }

        for handle in handles
        {
            //wait for all threads to finish before moving on
            handle.join().unwrap();
        }

        characterArray = tempCharacterArray;

        //Not all threads sent something, so we need to drop the ones that are still open
        drop(end_tx);
        for received in end_rx
        {
            if received == true
            {
                end = true;
                break;
            }
        }

        drop(wizard_tx);
        for received in wizard_rx
        {
            if received == true
            {
                wizard_shuffle = true;
                break;
            }
        }

        //printing game board
        let grid1 = Arc::clone(&grid);
        let grid2 = grid1.lock().unwrap();
        for (_i, row) in grid2.iter().enumerate()
        {
            for (_j, col) in row.iter().enumerate()
            {
                print!("{}  ", col);
            }
            println!()
        }
        println!();
        if end {break};
    }
}


fn character_move(mut character:(i8,i8,char,i8,i8,bool)) -> (i8,i8,char,i8,i8,bool) {
    let mut rng = rand::thread_rng();
    let mut row = rng.gen_range(0, 9);
    let mut column = rng.gen_range(0, 10);
    // setting current position for later reference
    character.3 = character.0;
    character.4 = character.1;
    //arbitrary -- just a way to randomize movement
    if row > column
    {
        //arbitrary -- just a way to randomize movement
        if row > 4 { row = 1;}
        else { row = -1;}
        if character.1 + row < 0 || character.1 + row > 3
        {
            character.1 = character.1 - row;
        }
        else
        {
            character.1 = character.1 + row;
        }
    }
    else
    {
        if column > 4 { column = 1;}
        else { column = -1;}
        if character.0 + column < 0 || character.0 + column > 3
        {
            character.0 = character.0 - column;
        }
        else
        {
            character.0 = character.0 + column;
        }
    }
    character
}