def filter_messages(messages):
    removed_dang = []
    cont_dang = []

    for message in messages:
        split_msg = message.split()
        words = []
        count = 0
        for word in split_msg:
            if "dang" == word.lower():
                count = count + 1
            else
                words.append(word)
        removed_dang.append(" ".join(words))
        cont_dang.append(count)

    return removed_dang, cont_dang